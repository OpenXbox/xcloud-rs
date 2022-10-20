use gamestreaming_webrtc::api::SessionResponse;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::math_rand_alpha;
use webrtc::peer_connection::offer_answer_options::RTCOfferOptions;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;

use gamestreaming_webrtc::{GamestreamingClient, Platform};
use xal::utils::TokenStore;

const TOKENS_FILEPATH: &str = "tokens.json";

pub trait GssvChannel {
    fn start(&self);
    fn on_open(&self);
    fn on_close(&self);
    fn on_message(&self);
}

struct ControlChannel;

impl GssvChannel for ControlChannel {
    fn start(&self) {
        todo!()
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn on_message(&self) {
        todo!()
    }
}

struct InputChannel;

impl GssvChannel for InputChannel {
    fn start(&self) {
        todo!()
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn on_message(&self) {
        todo!()
    }
}

struct MessageChannel;

impl GssvChannel for MessageChannel {
    fn start(&self) {
        todo!()
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn on_message(&self) {
        todo!()
    }
}

struct ChatChannel;

impl GssvChannel for ChatChannel {
    fn start(&self) {
        todo!()
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn on_message(&self) {
        todo!()
    }
}

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
    static ref PENDING_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
    static ref ADDRESS: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref GATHERED_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> = Arc::new(Mutex::new(vec![]));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ts = match TokenStore::load(TOKENS_FILEPATH) {
        Ok(ts) => ts,
        Err(err) => {
            println!("Failed to load tokens!");
            return Err(err);
        }
    };

    let xcloud = GamestreamingClient::create(
        Platform::Cloud,
        &ts.gssv_token.token_data.token,
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
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

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
                            let mut cs = pending_candidates3.lock().await;
                            cs.push(c);
                        } else {
                            // Candidate ready
                            println!("Candidate ready: {}", c);
                            let mut cs = candidates2.lock().await;
                            cs.push(c);
                        }
                    }
                }
            })
        }))
        .await;

    /*
    'chat': {
        id: 6,
        protocol: 'chatV1',
    },
    */
    let _chat_channel = ChatChannel {};
    let chat_channel = peer_connection
        .create_data_channel(
            "chat",
            Some(RTCDataChannelInit {
                protocol: Some("chatV1".to_owned()),
                ..Default::default()
            }),
        )
        .await?;

    /*
        'control': {
            id: 4,
            protocol: 'controlV1',
        },
    */

    let _control_channel = ControlChannel {};
    let control_channel = peer_connection
        .create_data_channel(
            "control",
            Some(RTCDataChannelInit {
                protocol: Some("controlV1".to_owned()),
                ..Default::default()
            }),
        )
        .await?;

    /*
        'input': {
            id: 3,
            ordered: true,
            protocol: '1.0',
        },
    */
    let _input_channel = InputChannel {};
    let input_channel = peer_connection
        .create_data_channel(
            "input",
            Some(RTCDataChannelInit {
                ordered: Some(true),
                protocol: Some("1.0".to_owned()),
                ..Default::default()
            }),
        )
        .await?;

    /*
    'message': {
        id: 5,
        protocol: 'messageV1',
    },
    */
    let _message_channel = MessageChannel {};
    let message_channel = peer_connection
        .create_data_channel(
            "message",
            Some(RTCDataChannelInit {
                protocol: Some("messageV1".to_owned()),
                ..Default::default()
            }),
        )
        .await?;

    // Allow us to receive 1 audio track, and 1 video track
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Audio,
            &[RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Sendrecv,
                send_encodings: vec![],
            }],
        )
        .await?;
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Video,
            &[RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                send_encodings: vec![],
            }],
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
        }))
        .await;

    /* KEEPME: Reference
    // Register channel opening handling
    let d1 = Arc::clone(&data_channel);
    data_channel.on_open(Box::new(move || {
        println!("Data channel '{}'-'{}' open. Random messages will now be sent to any connected DataChannels every 5 seconds", d1.label(), d1.id());

        let d2 = Arc::clone(&d1);
        Box::pin(async move {
            let mut result = Result::<usize, webrtc::Error>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(5));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        let message = math_rand_alpha(15);
                        println!("Sending '{}'", message);
                        result = d2.send_text(message).await.map_err(Into::into);
                    }
                };
            }
        })
    })).await;

    // Register text message handling
    let chat_label = chat_channel.label().to_owned();
    chat_channel
        .on_message(Box::new(move |msg: DataChannelMessage| {
            let msg = match String::from_utf8(msg.data.to_vec()) {
                Ok(str) => {
                    str
                },
                _ => {
                    format!("Binary={:?}", msg.data)
                }
            };
            println!("Message from DataChannel '{}': '{}'", chat_label, msg_str);
            Box::pin(async {})
        }))
        .await;
    */

    // Register text message handling
    let chat_label = chat_channel.label().to_owned();
    chat_channel
        .on_message(Box::new(move |msg: DataChannelMessage| {
            let msg_str = match String::from_utf8(msg.data.to_vec()) {
                Ok(str) => {
                    str
                },
                _ => {
                    format!("Binary={:?}", msg.data)
                }
            };
            println!("Message from DataChannel '{}': '{}'", chat_label, msg_str);
            Box::pin(async {})
        }))
        .await;

    let control_label = control_channel.label().to_owned();
    control_channel
        .on_message(Box::new(move |msg: DataChannelMessage| {
            let msg_str = match String::from_utf8(msg.data.to_vec()) {
                Ok(str) => {
                    str
                },
                _ => {
                    format!("Binary={:?}", msg.data)
                }
            };
            println!("Message from DataChannel '{}': '{}'", control_label, msg_str);
            Box::pin(async {})
        }))
        .await;

    let input_label = input_channel.label().to_owned();
    input_channel
        .on_message(Box::new(move |msg: DataChannelMessage| {
            let msg_str = match String::from_utf8(msg.data.to_vec()) {
                Ok(str) => {
                    str
                },
                _ => {
                    format!("Binary={:?}", msg.data)
                }
            };
            println!("Message from DataChannel '{}': '{}'", input_label, msg_str);
            Box::pin(async {})
        }))
        .await;

    let message_label = message_channel.label().to_owned();
    message_channel
        .on_message(Box::new(move |msg: DataChannelMessage| {
            let msg_str = match String::from_utf8(msg.data.to_vec()) {
                Ok(str) => {
                    str
                },
                _ => {
                    format!("Binary={:?}", msg.data)
                }
            };
            println!("Message from DataChannel '{}': '{}'", message_label, msg_str);
            Box::pin(async {})
        }))
        .await;

    // Create an offer to send to the other process
    let offer = peer_connection.create_offer(None).await?;
    let sdp_offer_string = offer.clone().sdp;
    // Sets the LocalDescription, and starts our UDP listeners
    // Note: this will start the gathering of ICE candidates
    peer_connection.set_local_description(offer).await?;

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
        let r = c.to_json().await?;
        candidates_ready.push(r);
    }
    let ice_response = xcloud.exchange_ice(&session, candidates_ready).await?;
    println!("ICE Response {:?}", ice_response);

    println!("Adding remote ICE candidates");
    for candidate in ice_response.exchange_response {
        println!("Adding remote ICE candidate={:?}", candidate);
        if candidate.candidate.contains("end-of-candidates") {
            println!("End of candidates, jumping out");
            break;
        }
        peer_connection.add_ice_candidate(candidate).await?;
    }

    println!("Press ctrl-c to stop");
    tokio::select! {
        _ = done_rx.recv() => {
            println!("received done signal!");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("");
        }
    };

    peer_connection.close().await?;

    Ok(())
}
