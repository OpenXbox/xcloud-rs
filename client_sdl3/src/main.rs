use anyhow::{Result, anyhow};
use log;
use simple_logger;
use ffmpeg_next::{codec, frame};

use std::ptr;
use std::mem::zeroed;
use std::ffi::CStr;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::Duration;
use gamestreaming_webrtc::webrtc::rtp::codecs::h264::H264Packet;
use gamestreaming_webrtc::webrtc::rtp::packetizer::Depacketizer;
use gamestreaming_webrtc::webrtc::api::interceptor_registry::register_default_interceptors;
use gamestreaming_webrtc::webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
use gamestreaming_webrtc::webrtc::api::APIBuilder;
use gamestreaming_webrtc::webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use gamestreaming_webrtc::webrtc::ice_transport::ice_server::RTCIceServer;
use gamestreaming_webrtc::webrtc::interceptor::registry::Registry;
use gamestreaming_webrtc::webrtc::peer_connection::configuration::RTCConfiguration;
use gamestreaming_webrtc::webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use gamestreaming_webrtc::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use gamestreaming_webrtc::webrtc::peer_connection::RTCPeerConnection;
use gamestreaming_webrtc::webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use gamestreaming_webrtc::webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use gamestreaming_webrtc::webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use gamestreaming_webrtc::webrtc::rtp_transceiver::{RTCPFeedback, RTCRtpTransceiverInit};
use gamestreaming_webrtc::webrtc::track::track_remote::TrackRemote;

use gamestreaming_webrtc::api::{IceCandidate, SessionResponse};
use gamestreaming_webrtc::{GamestreamingClient, Platform};
use gamestreaming_webrtc::auth::{authenticate, GamestreamingAuthContext};
use gamestreaming_webrtc::channels::{GssvChannel, GssvChannelInit, ChatChannel, ControlChannel, InputChannel, MessageChannel};

use sdl3_sys::everything::*;

#[macro_use]
extern crate lazy_static;

const TOKENS_FILEPATH: &str = "tokens.json";


lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
    static ref PENDING_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
    static ref ADDRESS: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref GATHERED_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
}


const WINDOW_WIDTH: i32 = 1920;
const WINDOW_HEIGHT: i32 = 1080;

const NALU_TTYPE_STAP_A: u32 = 24;
const NALU_TTYPE_SPS: u32 = 7;
const NALU_TYPE_BITMASK: u32 = 0x1F;

fn is_key_frame(data: &[u8]) -> bool {
    if data.len() < 4 {
        false
    } else {
        let word = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let nalu_type = (word >> 24) & NALU_TYPE_BITMASK;
        (nalu_type == NALU_TTYPE_STAP_A && (word & NALU_TYPE_BITMASK) == NALU_TTYPE_SPS)
            || (nalu_type == NALU_TTYPE_SPS)
    }
}

async fn create_peer_connection() -> Result<RTCPeerConnection, gamestreaming_webrtc::webrtc::Error> {
    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m)?;

    // Create the API object with the MediaEngine
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Create a new RTCPeerConnection
    api.new_peer_connection(config).await
}

async fn start_session(ts: GamestreamingAuthContext, platform: Platform) -> Result<(GamestreamingClient, SessionResponse)> {
    let xcloud = GamestreamingClient::new(
        platform.clone(),
        &ts.gssv_token.token,
        &ts.xcloud_transfer_token.lpt,
    )
    .await?;

    let session = match platform {
        Platform::Cloud => {
            match xcloud.lookup_games().await?.first() {
                Some(title) => {
                    log::info!("Starting title: {:?}", title);
                    let session = xcloud.start_stream_xcloud(&title.title_id).await?;
                    log::info!("Session started successfully: {:?}", session);
        
                    session
                }
                None => {
                    return Err(anyhow!("No titles found"));
                }
            }
        },
        Platform::Home => {
            match xcloud.lookup_consoles().await {
                Ok(consoles) => {
                    let c = consoles.results.first().unwrap();
                    xcloud.start_stream_xhome(&c.server_id).await?
                },
                Err(err) => {
                    return Err(anyhow!("No consoles received from API, error: {err}"));
                }
            }
        },
    };

    Ok((xcloud, session))
}

async fn start_remote_connection(
    audio_tx: tokio::sync::mpsc::UnboundedSender<Arc<TrackRemote>>,
    video_tx: tokio::sync::mpsc::UnboundedSender<Arc<TrackRemote>>
) -> Result<()> {
    let ts = authenticate(TOKENS_FILEPATH)
        .await
        .map_err(|e|anyhow!("Authentication failed, error: {e:?}"))?;

    let platform = Platform::Cloud;
    //let platform = Platform::Home;

    let (xcloud, session) = start_session(ts, platform).await?;

    // WebRTC part

    // Create a new RTCPeerConnection
    let peer_connection: Arc<RTCPeerConnection> = Arc::new(create_peer_connection().await?);

    // When an ICE candidate is available send to the other Pion instance
    // the other Pion instance will add this candidate by calling AddICECandidate
    let pc = Arc::downgrade(&peer_connection);
    let pending_candidates2 = Arc::clone(&PENDING_CANDIDATES);
    let candidates = Arc::clone(&GATHERED_CANDIDATES);
    peer_connection
        .on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            log::debug!("on_ice_candidate {:?}", c);
            let candidates2 = Arc::clone(&candidates);
            let pc2 = pc.clone();
            let pending_candidates3 = Arc::clone(&pending_candidates2);
            Box::pin(async move {
                if let Some(c) = c {
                    if let Some(pc) = pc2.upgrade() {
                        let desc = pc.remote_description().await;
                        if desc.is_none() {
                            // Candidate pending
                            log::debug!("Candidate pending: {}", c);
                            let mut cs_pending = pending_candidates3.lock().await;
                            cs_pending.push(c);
                        } else {
                            // Candidate ready
                            log::debug!("Candidate ready: {}", c);
                            let mut cs_ready = candidates2.lock().await;
                            cs_ready.push(c);
                        }
                    }
                }
            })
        }));

    peer_connection.on_data_channel(Box::new(move |c| {
        log::warn!("On data channel: {:?}", c.label());
        Box::pin(async {})
    }));

    peer_connection.on_ice_connection_state_change(Box::new(move |c| {
        log::warn!("On ice connection state change: {:?}", c);
        Box::pin(async {})
    }));
    peer_connection.on_ice_gathering_state_change(Box::new(move |c| {
        log::warn!("On ice gathering state change: {:?}", c);
        Box::pin(async {})
    }));
    peer_connection.on_negotiation_needed(Box::new(move || {
        log::warn!("On negotiation needed");
        Box::pin(async {})
    }));
    peer_connection.on_signaling_state_change(Box::new(move |c| {
        log::warn!("On signaling state change: {:?}", c);
        Box::pin(async {})
    }));

    // Create channels and assign on-open and on-message callbacks
    let channel_input = InputChannel::init(peer_connection.clone()).await?;
    let channel_control = ControlChannel::init(peer_connection.clone()).await?;
    let channel_message = MessageChannel::init(peer_connection.clone()).await?;
    let _channel_chat = ChatChannel::init(peer_connection.clone()).await?;

    // Allow us to receive 1 audio track, and 1 video track
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Audio,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Sendrecv,
                send_encodings: vec![],
            }),
        )
        .await?;
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Video,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                send_encodings: vec![],
            }),
        )
        .await?;

    let (done_tx, _done_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection
        .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            log::info!("Peer Connection State has changed: {}", s);

            if s == RTCPeerConnectionState::Failed {
                // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                log::error!("Peer Connection has gone to failed exiting");
                let _ = done_tx.try_send(());
            }

            Box::pin(async {})
        }));

    let _notify_tx = Arc::new(Notify::new());

    let pc = Arc::downgrade(&peer_connection);
    peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
        // Send a PLI on an interval so that the publisher is pushing a keyframe every rtcpPLIInterval
        let media_ssrc = track.ssrc();
        let pc2 = pc.clone();
        tokio::spawn(async move {
            let mut result = Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(3));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        if let Some(pc) = pc2.upgrade(){
                            result = pc.write_rtcp(&[Box::new(PictureLossIndication{
                                sender_ssrc: 0,
                                media_ssrc,
                            })]).await.map_err(Into::into);
                        }else {
                            break;
                        }
                    }
                };
            }
        });

        let video_sender = video_tx.clone();
        let audio_sender = audio_tx.clone();

        match track.kind() {
            RTPCodecType::Video => {
                let _ = video_sender.send(track);
            }
            RTPCodecType::Audio => {
                let _ = audio_sender.send(track);
            }
            _ => {}
        }

        Box::pin(async {})
    }));

    // Create an offer to send to the other process
    let offer = peer_connection.create_offer(None).await?;
    let sdp_offer_string = offer.clone().sdp;
    // Sets the LocalDescription, and starts our UDP listeners
    // Note: this will start the gathering of ICE candidates
    peer_connection.set_local_description(offer).await?;

    // Xcloud
    let sdp_response = xcloud.exchange_sdp(&session, &sdp_offer_string).await?;
    log::debug!("SDP Response {:?}", sdp_response);

    match sdp_response.exchange_response.sdp {
        Some(sdp) => {
            log::debug!("Setting SDP answer...");
            let answer = RTCSessionDescription::answer(sdp)?;
            log::debug!("SDP answer: {:?}", answer);
            if let Err(sdp_fail) = peer_connection.set_remote_description(answer).await {
                log::error!("Failed to set remote SDP answer: {:?}", sdp_fail);
                return Err(sdp_fail.into());
            }
        }
        None => {
            peer_connection.close().await?;
            return Err(anyhow!("Failed to get successful SDP answer"));
        }
    }

    let cs = PENDING_CANDIDATES.lock().await;
    let css = cs.to_vec();
    let mut candidates_ready = vec![];

    for c in css {
        let json = c.to_json()?;
        let r = IceCandidate {
            candidate: json.candidate,
            sdp_mid: json.sdp_mid,
            sdp_mline_index: json.sdp_mline_index,
            username_fragment: json.username_fragment,
        };
        candidates_ready.push(r);
    }

    // Xcloud
    let ice_response = xcloud.exchange_ice(&session, candidates_ready).await?;
    log::debug!("ICE Response {:?}", ice_response);

    if ice_response.exchange_response.is_empty()  {
        return Err(anyhow!("No candidates in ICE response"));
    }

    log::debug!("Adding remote ICE candidates");
    for candidate in ice_response.exchange_response {
        log::debug!("Adding remote ICE candidate={:?}", candidate);
        if candidate.candidate.contains("end-of-candidates") {
            log::debug!("End of candidates, jumping out");
            break;
        }
        let c = RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: candidate.sdp_mid,
            sdp_mline_index: candidate.sdp_mline_index,
            username_fragment: candidate.username_fragment,
        };
        peer_connection.add_ice_candidate(c).await?;
    }

    log::warn!("Waiting for handshake ack on message channel");
    let mut handshake_rx = channel_message.handshake_ack_rx.lock().await;
    while handshake_rx.recv().await.is_none() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    drop(handshake_rx);

    log::warn!("[+] Handshake ack received, starting channels...");
    channel_input.start().await.unwrap();
    channel_control.start().await.unwrap();
    log::warn!("[+] channels started, looping keepalive msg now...");

    loop {
        if let Err(e) = xcloud.keepalive(&session).await {
            log::error!("Keepalive failed, error: {e:?}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    let do_decoding = true;

    simple_logger::init_with_level(log::Level::Debug)?;

    // XCloud part
    let mut window = unsafe { zeroed() };
    let mut renderer = unsafe { zeroed() };
    let mut texture = unsafe { zeroed() };
    let mut audiostream = unsafe { zeroed() };

    unsafe {
        let audiospec = SDL_AudioSpec {
            format: SDL_AUDIO_F32,
            channels: 2,
            freq: 48000
        };

        if SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO | SDL_INIT_GAMEPAD) == false {
            log::error!("SDL_Init Error: {:?}", CStr::from_ptr(SDL_GetError()));
            return Err(anyhow!("SDL Init failed"));
        }

        // Create a window
        window = SDL_CreateWindow(
            c"SDL3 Video Playback".as_ptr(),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            SDL_WindowFlags::default()
        );
        
        if window.is_null() {
            log::error!("SDL_CreateWindow Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateWindow Error"));
        }

        renderer = SDL_CreateRenderer(window, ptr::null());

        if renderer.is_null() {
            log::error!("SDL_CreateRenderer Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateRenderer Error"));
        }

        texture = SDL_CreateTexture(renderer, SDL_PIXELFORMAT_IYUV, SDL_TEXTUREACCESS_STREAMING, WINDOW_WIDTH, WINDOW_HEIGHT);
        if texture.is_null() {
            log::error!("SDL_CreateTexture Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateTexture Error"));
        }

        audiostream = SDL_OpenAudioDeviceStream(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &audiospec, None, ptr::null_mut());
        if audiostream.is_null() {
            log::error!("SDL_OpenAudioDeviceStream Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_OpenAudioDeviceStream Error"));
        }
    }

    let (video_tx, mut video_rx) = tokio::sync::mpsc::unbounded_channel();
    let (audio_tx, mut audio_rx) = tokio::sync::mpsc::unbounded_channel();

    log::debug!("Spawning remote connection...");
    let _handle = tokio::spawn(start_remote_connection(audio_tx, video_tx));


    // Create a channel to signal the main loop to exit
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::channel::<()>(1);
    log::info!("Press ctrl-c to stop");

    ffmpeg_next::init()?;

    let mut video_decoder = codec::decoder::new()
        .open_as(codec::decoder::find(codec::Id::H264))
        .unwrap()
        .video()
        .unwrap();

    let mut audio_decoder = codec::decoder::new()
        .open_as(codec::decoder::find(codec::Id::OPUS))
        .unwrap()
        .audio()
        .unwrap();

    unsafe {
        // Clear the screen
        SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
        SDL_RenderClear(renderer);

        let (mut got_audio, mut got_video) = (false, false);
        let mut has_keyframe = false;
        let mut h264pkt = H264Packet::default();
        let mut event: SDL_Event = zeroed();

        let mut video_track = None;
        let mut audio_track = None;

        'running: loop {
            while SDL_PollEvent(&mut event as *mut _) {
                match event.r#type {
                    0x100 => {
                        let _ = exit_tx.send(()).await;
                    },
                    _evt_type => {
                        // log::info!("Unhandled evt: {}", evt_type.0);
                    }
                }

            }

            if let Ok(a_track) = audio_rx.try_recv() {
                audio_track.replace(a_track);

                log::info!("Got audio track...");
            }

            if let Ok(v_track) = video_rx.try_recv() {
                video_track.replace(v_track);

                log::info!("Got video track...");
            }

            if let Some(ref track) = audio_track {
                if let Ok((rtp_packet, _b)) = track.read_rtp().await {
                    if do_decoding {
                        if !rtp_packet.payload.is_empty() {
                            let payload = rtp_packet.payload;
                            if !got_audio {
                                got_audio = true;
                                log::info!("Got first audio frame");
                                if !SDL_ResumeAudioStreamDevice(audiostream) {
                                    log::error!("Failed to unpause audio stream");
                                }
                            }
                            let mut pkt = ffmpeg_next::Packet::copy(&payload);
            
                            if audio_decoder.send_packet(&mut pkt).is_ok() {
                                let mut frame = frame::Audio::empty();
                                while audio_decoder.receive_frame(&mut frame).is_ok() {
                                    if !SDL_PutAudioStreamData(
                                        audiostream,
                                        frame.data(0).as_ptr() as *mut _,
                                        frame.data(0).len() as i32
                                    ) {
                                        log::error!("Failed to put data into audio stream");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(ref track) = video_track {
                if let Ok((rtp_packet, _b)) = track.read_rtp().await {
                    if do_decoding {
                        if !rtp_packet.payload.is_empty() {
                            let payload = rtp_packet.payload;
                            if !got_video {
                                got_video = true;
                                log::info!("Got first video frame");
                            }
            
                            if !has_keyframe {
                                has_keyframe = is_key_frame(&payload);
                            }
            
                            if has_keyframe {
                                if let Ok(data) = h264pkt.depacketize(&payload) {
                                    let mut pkt = ffmpeg_next::Packet::copy(&data);
            
                                    if video_decoder.send_packet(&mut pkt).is_ok() {
                                        let mut frame = frame::Video::empty();
                                        while video_decoder.receive_frame(&mut frame).is_ok() {
                                            SDL_UpdateYUVTexture(
                                                texture,
                                                ptr::null(),
                                                frame.data(0).as_ptr(),
                                                frame.plane_width(0) as i32,
                                                frame.data(1).as_ptr(),
                                                frame.plane_width(1) as i32,
                                                frame.data(2).as_ptr(),
                                                frame.plane_width(2) as i32
                                            );
                        
                                            SDL_RenderTexture(renderer, texture, ptr::null(), ptr::null());
                        
                                            // Present the back buffer
                                            SDL_RenderPresent(renderer);
                                        }
                                    }
                                }
                            }    
                        }
                    }
                }
            }

            if let Ok(_exit_signal) = exit_rx.try_recv() {
                log::info!("Received exit signal, exiting loop");
                break 'running;
            }
        }

        SDL_DestroyRenderer(renderer);
        SDL_DestroyWindow(window);
        SDL_Quit();
    }

    Ok(())
}
