use anyhow::Result;

use std::fs::File;
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

use std::ptr;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // XCloud part

    let ts = authenticate(TOKENS_FILEPATH).await?;

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

    let (video_file, audio_file) = ("video.mkv", "audio.ogg");

    let h264_writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>> =
        Arc::new(Mutex::new(H264Writer::new(File::create(video_file)?)));
    let ogg_writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>> = Arc::new(Mutex::new(
        OggWriter::new(File::create(audio_file)?, 48000, 2)?,
    ));

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
            return Err("Failed to get successful SDP answer".into());
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

    println!("Press ctrl-c to stop");

    unsafe {
        if SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO) == false {
            println!("SDL_Init Error: {:?}", SDL_GetError());
            return Err("SDL Init failed".into());
        }

        // Create a window
        let window = SDL_CreateWindow(
            b"SDL3 Video Playback\0".as_ptr() as *const i8,
            0,
            0,
            SDL_WINDOWPOS_CENTERED as u64
        );

        if window.is_null() {
            println!("SDL_CreateWindow Error: {:?}", SDL_GetError());
            SDL_Quit();
            return Err("SDL_CreateWindow Error".into());
        }

         // Create a renderer
        let renderer = SDL_CreateRenderer(
            window,
            "Video renderer".as_ptr() as *mut _
        );
        if renderer.is_null() {
            println!("SDL_CreateRenderer Error: {:?}", SDL_GetError());
            SDL_DestroyWindow(window);
            SDL_Quit();
            return Err("SDL_CreateRenderer".into());
        }

        // Create a channel to signal the main loop to exit
        let (exit_tx, mut exit_rx) = tokio::sync::mpsc::channel::<()>(1);

        //Event loop
        let mut event: SDL_Event = std::mem::zeroed();
        let mut frame_buffer: Vec<u8> = vec![0; (WINDOW_WIDTH * WINDOW_HEIGHT * 3 / 2) as usize];

        'running: loop {
            tokio::select! {
                result = track.read_rtp() => {
                    match result {
                        Ok((rtp_packet, _)) => {
                            // TODO: Implement H.264 decoding and render to texture
                            // For now, just fill the framebuffer with a color
                            frame_buffer.fill(128);

                            // Example: Directly copy RTP payload to framebuffer (for raw YUV420p)
                            // This assumes the RTP payload is raw YUV420p data
                            frame_buffer.copy_from_slice(&rtp_packet.payload);

                            // Update texture with new data
                            texture.update(None, &frame_buffer, WINDOW_WIDTH as usize * 3 / 2).unwrap();

                            // Clear the screen
                            SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
                            SDL_RenderClear(renderer);

                            // Copy the texture to the screen
                            //SDL_RenderCopy(renderer, texture, None, None).unwrap();

                            // Present the back buffer
                            SDL_RenderPresent(renderer);
                        }
                        Err(err) => {
                            println!("Error reading RTP packet: {}", err);
                            break 'running;
                        }
                    }
                }
                _ = notify_rx.notified() => {
                    println!("Received notification, exiting loop");
                    break 'running;
                }
                _ = exit_rx.recv() => {
                    println!("Received exit signal, exiting loop");
                    break 'running;
                }
                _ = tokio::task::spawn_blocking(move || {
                    if SDL_PollEvent(&mut event) {
                        match event.r#type {
                            SDL_QUIT => {
                                println!("SDL_QUIT received, sending exit signal");
                                let _ = exit_tx.try_send(());
                            }
                            SDL_KEYDOWN => {
                                if event.key.key == SDLK_ESCAPE {
                                    println!("SDLK_ESCAPE received, sending exit signal");
                                    let _ = exit_tx.try_send(());
                                }
                            }
                            _ => {}
                        }
                    }
                }) => {}
            }
        }

        SDL_DestroyRenderer(renderer);
        SDL_DestroyWindow(window);
        SDL_Quit();
    }

    Ok(())
}
