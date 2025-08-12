use anyhow::{Result, anyhow};
use ffmpeg_next::{codec, filter, format, frame, media};
use webrtc::rtp::codecs::h264::H264Packet;
use webrtc::rtp::packetizer::Depacketizer;
use tokio::io::{AsyncWrite,AsyncWriteExt};

use std::alloc::{self, alloc};
use std::any::Any;
use std::ptr;
use std::io::Write;
use std::mem::zeroed;
use std::ffi::CStr;
use std::fs::File;
use bytes::Bytes;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, Notify};
use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::h264_writer::H264Writer;
use webrtc::media::io::ogg_writer::OggWriter;
use webrtc::media::io::h264_reader::H264Reader;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::track::track_remote::TrackRemote;

use gamestreaming_webrtc::api::IceCandidate;
use gamestreaming_webrtc::{GamestreamingClient, Platform};
use gamestreaming_webrtc::auth::authenticate;

use sdl3_sys::everything::*;

#[macro_use]
extern crate lazy_static;

const TOKENS_FILEPATH: &str = "tokens.json";

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
struct DataChannelParams {
    id: i32,
    protocol: &'static str,
    is_ordered: Option<bool>,
}


lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
    static ref PENDING_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
    static ref ADDRESS: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref GATHERED_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
}


const WINDOW_WIDTH: i32 = 640;
const WINDOW_HEIGHT: i32 = 480;

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

async fn save_to_disk(
    track: Arc<TrackRemote>,
    notify: Arc<Notify>,
) -> Result<()> {
    println!("Exited loop, cleaning up SDL");
    Ok(())
}

async fn create_peer_connection() -> Result<RTCPeerConnection, webrtc::Error> {
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
    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 102,
            ..Default::default()
        },
        RTPCodecType::Video,
    )?;

    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 111,
            ..Default::default()
        },
        RTPCodecType::Audio,
    )?;

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

async fn start_remote_connection(audio_tx: tokio::sync::mpsc::Sender<Bytes>, video_tx: tokio::sync::mpsc::Sender<Bytes>) -> Result<()> {
    let ts = authenticate(TOKENS_FILEPATH)
        .await
        .map_err(|e|anyhow!("Authentication failed"))?;

    let xcloud = GamestreamingClient::new(
        Platform::Home,
        &ts.gssv_token.token,
        &ts.xcloud_transfer_token.lpt,
    )
    .await?;

    let session = match xcloud.lookup_consoles().await {
        Ok(consoles) => {
            let c = consoles.results.first().unwrap();
            xcloud.start_stream_xhome(&c.server_id).await?
        },
        Err(err) => {
            return Err(anyhow!("No consoles received from API"));
        }
    };


    /*
    let xcloud = GamestreamingClient::new(
        Platform::Cloud,
        &ts.gssv_token.token,
        &ts.xcloud_transfer_token.lpt,
    )
    .await?;
    let session = match xcloud.lookup_games().await?.first() {
        Some(title) => {
            println!("Starting title: {:?}", title);
            let session = xcloud.start_stream_xcloud(&title.title_id).await?;
            println!("Session started successfully: {:?}", session);

            session
        }
        None => {
            return Err("No titles received from API".into());
        }
    };
    */

    // WebRTC part

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(create_peer_connection().await?);

    // When an ICE candidate is available send to the other Pion instance
    // the other Pion instance will add this candidate by calling AddICECandidate
    let pc = Arc::downgrade(&peer_connection);
    let pending_candidates2 = Arc::clone(&PENDING_CANDIDATES);
    let candidates = Arc::clone(&GATHERED_CANDIDATES);
    peer_connection
        .on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            println!("on_ice_candidate {:?}", c);
            let candidates2 = Arc::clone(&candidates);
            let pc2 = pc.clone();
            let pending_candidates3 = Arc::clone(&pending_candidates2);
            Box::pin(async move {
                if let Some(c) = c {
                    if let Some(pc) = pc2.upgrade() {
                        let desc = pc.remote_description().await;
                        if desc.is_none() {
                            // Candidate pending
                            println!("Candidate pending: {}", c);
                            let mut cs_pending = pending_candidates3.lock().await;
                            cs_pending.push(c);
                        } else {
                            // Candidate ready
                            println!("Candidate ready: {}", c);
                            let mut cs_ready = candidates2.lock().await;
                            cs_ready.push(c);
                        }
                    }
                }
            })
        }));

    let channel_params: HashMap<String, DataChannelParams> = [
        ("input".into(), DataChannelParams { id: 3, protocol: "1.0".into(), is_ordered: Some(true) }),
        ("control".into(), DataChannelParams { id: 4, protocol: "controlV1".into(), is_ordered: None }),
        ("message".into(), DataChannelParams { id: 5, protocol: "messageV1".into(), is_ordered: None }),
        ("chat".into(), DataChannelParams { id: 6, protocol: "chatV1".into(), is_ordered: None }),

    ].into();

    let mut channel_defs: HashMap<String, Arc<RTCDataChannel>> = HashMap::new();
    // Create channels and store in HashMap
    for (name, params) in channel_params.into_iter() {
        let chan = peer_connection
        .create_data_channel(
            &name,
            Some(RTCDataChannelInit {
                ordered: params.is_ordered,
                protocol: Some(params.protocol.to_owned()),
                ..Default::default()
            }),
        )
        .await?;

        channel_defs.insert(name, chan);
    }

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

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection
        .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {}", s);

            if s == RTCPeerConnectionState::Failed {
                // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                println!("Peer Connection has gone to failed exiting");
                let _ = done_tx.try_send(());
            }

            Box::pin(async {})
        }));

    // Register channel opening / on message handling

    for (name, channel) in channel_defs.into_iter() {
        let d1 = Arc::clone(&channel);
        channel.on_open(Box::new(move || {
            println!("Data channel '{}'-'{}' open", d1.label(), d1.id());

            Box::pin(async move {
                let result = Result::<usize, webrtc::Error>::Ok(0);
                while result.is_ok() {
                    let timeout = tokio::time::sleep(Duration::from_secs(5));
                    tokio::pin!(timeout);

                    tokio::select! {
                        _ = timeout.as_mut() =>{
                            /*
                            From example code - Sending random strings over datachannel
                            let message = math_rand_alpha(15);
                            println!("Sending '{}'", message);
                            result = d2.send_text(message).await.map_err(Into::into);
                            */
                        }
                    };
                }
            })
        }));

        let message_label = name.clone();
        channel
            .on_message(Box::new(move |msg: DataChannelMessage| {
                let msg_str = match String::from_utf8(msg.data.to_vec()) {
                    Ok(str) => str,
                    _ => {
                        format!("Binary={:?}", msg.data)
                    }
                };
                println!(
                    "Message from DataChannel '{}': '{}'",
                    message_label, msg_str
                );
                Box::pin(async {})
            }));
    }

    /*
    let (video_file, audio_file) = ("video.mkv", "audio.ogg");

    let h264_writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>> =
        Arc::new(Mutex::new(H264Writer::new(File::create(video_file)?)));
    let ogg_writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>> = Arc::new(Mutex::new(
        OggWriter::new(File::create(audio_file)?, 48000, 2)?,
    ));
    */

    let notify_tx = Arc::new(Notify::new());
    let notify_rx = notify_tx.clone();

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

        let (sender, filenamebase) = match track.kind() {
            RTPCodecType::Video => (video_tx.clone(), "video"),
            RTPCodecType::Audio => (audio_tx.clone(), "audio"),
            RTPCodecType::Unspecified => panic!("Unexpected track type!")
        };

        Box::pin(async move {
            loop {
                if track.kind() == RTPCodecType::Video {
                    if let Ok((a,_b)) = track.read_rtp().await {
                        if !a.payload.is_empty() {
                            let res = sender.send(a.payload).await;
                            if let Err(e) = res {
                                println!("Sending RTP packet, Error: {:?}", e);
                            }
                        }
                    }
                }
            }
        })
    }));

    // Create an offer to send to the other process
    let offer = peer_connection.create_offer(None).await?;
    let sdp_offer_string = offer.clone().sdp;
    // Sets the LocalDescription, and starts our UDP listeners
    // Note: this will start the gathering of ICE candidates
    peer_connection.set_local_description(offer).await?;

    // Xcloud
    let sdp_response = xcloud.exchange_sdp(&session, &sdp_offer_string).await?;
    println!("SDP Response {:?}", sdp_response);

    match sdp_response.exchange_response.sdp {
        Some(sdp) => {
            println!("Setting SDP answer...");
            let answer = RTCSessionDescription::answer(sdp)?;
            println!("SDP answer: {:?}", answer);
            if let Err(sdp_fail) = peer_connection.set_remote_description(answer).await {
                println!("Failed to set remote SDP answer: {:?}", sdp_fail);
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
    println!("ICE Response {:?}", ice_response);

    if ice_response.exchange_response.is_empty()  {
        return Err(anyhow!("No candidates in ICE response"));
    }

    println!("Adding remote ICE candidates");
    for candidate in ice_response.exchange_response {
        println!("Adding remote ICE candidate={:?}", candidate);
        if candidate.candidate.contains("end-of-candidates") {
            println!("End of candidates, jumping out");
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

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // XCloud part
    let mut window = unsafe { zeroed() };
    let mut renderer = unsafe { zeroed() };
    let mut texture = unsafe { zeroed() };

    unsafe {
        if SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO) == false {
            println!("SDL_Init Error: {:?}", CStr::from_ptr(SDL_GetError()));
            return Err(anyhow!("SDL Init failed"));
        }

        // Create a window
        window = SDL_CreateWindow(
            c"SDL3 Video Playback".as_ptr(),
            1920,
            1080,
            SDL_WindowFlags::default()
        );
        
        if window.is_null() {
            println!("SDL_CreateWindow Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateWindow Error"));
        }

        renderer = SDL_CreateRenderer(window, ptr::null());

        if renderer.is_null() {
            println!("SDL_CreateRenderer Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateRenderer Error"));
        }

        texture = SDL_CreateTexture(renderer, SDL_PIXELFORMAT_IYUV, SDL_TEXTUREACCESS_STREAMING, 1920, 1080);
        if texture.is_null() {
            println!("SDL_CreateTexture Error: {:?}", CStr::from_ptr(SDL_GetError()));
            SDL_Quit();
            return Err(anyhow!("SDL_CreateTexture Error"));
        }
    }

    let (mut video_tx, mut video_rx) = tokio::sync::mpsc::channel(10);
    let (mut audio_tx, mut audio_rx) = tokio::sync::mpsc::channel(10);

    println!("Spawning remote connection...");
    let handle = tokio::spawn(start_remote_connection(audio_tx, video_tx));


    // Create a channel to signal the main loop to exit
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::channel::<()>(1);
    println!("Press ctrl-c to stop");

    ffmpeg_next::init()?;

    let mut decoder = codec::decoder::new()
        .open_as(codec::decoder::find(codec::Id::H264))
        .unwrap()
        .video()
        .unwrap();

    unsafe {
        // Clear the screen
        SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
        SDL_RenderClear(renderer);

        let mut has_keyframe = false;
        let mut h264pkt = H264Packet::default();
        let mut evt: SDL_Event = zeroed();

        'running: loop {
            while SDL_PollEvent(&mut evt as *mut _) {
                match evt.r#type {
                    256 => {
                        println!("Key: {}", evt.key.key);
                        let _ = exit_tx.send(()).await;
                    },
                    _ => {
                        // println!("Unhandled evt: {:?}", evt.r#type);
                    }
                }

            }

            if let Ok(payload) = video_rx.try_recv() {
                if !has_keyframe {
                    has_keyframe = is_key_frame(&payload);
                }

                if has_keyframe {
                    if let Ok(data) = h264pkt.depacketize(&payload) {
                        let mut pkt = ffmpeg_next::Packet::copy(&data);

                        if decoder.send_packet(&mut pkt).is_ok() {
                            let mut frame = frame::Video::empty();
                            while decoder.receive_frame(&mut frame).is_ok() {
                                //println!("ok");
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

            if let Ok(exit_signal) = exit_rx.try_recv() {
                println!("Received exit signal, exiting loop");
                break 'running;
            }
        }

        SDL_DestroyRenderer(renderer);
        SDL_DestroyWindow(window);
        SDL_Quit();
    }

    Ok(())
}
