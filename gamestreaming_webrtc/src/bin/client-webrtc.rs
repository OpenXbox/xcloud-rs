use anyhow::Result;
use gamestreaming_webrtc::api::{IceCandidate, SessionResponse};
use gilrs::ff::{BaseEffect, EffectBuilder};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
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
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::track::track_remote::TrackRemote;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gst::{prelude::*, State, glib, ClockTime};

use gamestreaming_webrtc::{
    ChannelProxy, ChannelType, GamestreamingClient, Platform, DataChannelMsg,
    ChannelExchangeMsg, GssvClientEvent, GssvChannelEvent, GamepadData, GamepadProcessor
};
use xal::utils::TokenStore;
use gilrs::{Gilrs, Event};

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


async fn read_packet_into_gst_buf(track: &Arc<TrackRemote>) -> Result<gst::Buffer, Box<dyn std::error::Error>> {
    let mut data = [0u8; 1500];
    let (pkt_size, attrs) = track.read(&mut data).await?;
    let mut buffer = gst::Buffer::with_size(pkt_size).unwrap();
    {
        let buffer = buffer.get_mut().unwrap();
        buffer.copy_from_slice(0, &data[..pkt_size]).expect("Failed to fill buffer from slice");
    }

    Ok(buffer)
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
    // Gstreamer
    gst::init()?;

    // Gamepad lib
    let mut gilrs = Gilrs::new()?;

    let pipeline = gst::Pipeline::default();

    let caps_video = gst::Caps::from_str(
            &format!("application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, payload={}", 102)
        )
        .unwrap();
    let appsrc_video = gst_app::AppSrc::builder()
        .format(gst::Format::Time)
        .is_live(true)
        .do_timestamp(true)
        .caps(&caps_video)
        .build();
    let depay_video = gst::ElementFactory::make("rtph264depay").build()?;
    // let parse_video = gst::ElementFactory::make("h264parse").build()?;
    let decode_video = gst::ElementFactory::make("avdec_h264").build()?;
    let convert_video = gst::ElementFactory::make("videoconvert").build()?;
    let sink_video = gst::ElementFactory::make("autovideosink").build()?;

    pipeline.add_many(&[appsrc_video.upcast_ref(), &depay_video, /*&parse_video, */ &decode_video, &convert_video, &sink_video])?;
    gst::Element::link_many(&[appsrc_video.upcast_ref(), &depay_video, /*&parse_video, */ &decode_video, &convert_video, &sink_video])?;

    let caps_audio = gst::Caps::from_str(
            &format!("application/x-rtp, media=audio, clock-rate=48000, encoding-name=OPUS, payload={}", 111)
        )
        .unwrap();
    let appsrc_audio = gst_app::AppSrc::builder()
        .format(gst::Format::Time)
        .is_live(true)
        .do_timestamp(true)
        .caps(&caps_audio)
        .build();
    let depay_audio = gst::ElementFactory::make("rtpopusdepay").build()?;
    let parse_audio = gst::ElementFactory::make("opusparse").build()?;
    let decode_audio = gst::ElementFactory::make("avdec_opus").build()?;
    let convert_audio = gst::ElementFactory::make("audioconvert").build()?;
    let sink_audio = gst::ElementFactory::make("autoaudiosink").build()?;
    
    pipeline.add_many(&[appsrc_audio.upcast_ref(), &depay_audio, &parse_audio, &decode_audio, &convert_audio, &sink_audio])?;
    gst::Element::link_many(&[appsrc_audio.upcast_ref(), &depay_audio, &parse_audio, &decode_audio, &convert_audio, &sink_audio])?;

    pipeline.set_state(State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");


    // XCloud part

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
        }))
        .await;

    let (channel_tx, mut channel_rx) = mpsc::channel(10);
    let channel_proxy = Arc::new(Mutex::new(ChannelProxy::new(channel_tx)));
    let mut channel_defs: Arc<Mutex<HashMap<ChannelType, Arc<RTCDataChannel>>>> = Arc::new(Mutex::new(HashMap::new()));
    // Create channels and store in HashMap
    for (chan_type, params) in ChannelProxy::data_channel_create_params() {
        let channel = peer_connection
            .create_data_channel(
                &chan_type.to_string(),
                Some(RTCDataChannelInit {
                    ordered: params.is_ordered,
                    protocol: Some(params.protocol.to_owned()),
                    ..Default::default()
                }),
            )
            .await?;

        // Register channel open / close / message handling
        let channel_clone = channel.clone();
        let channel_proxy_clone = channel_proxy.clone();
            channel
            .on_open(Box::new(move || {
                let channel_proxy = channel_proxy_clone.clone();
                let channel = channel_clone.clone();
                Box::pin(async move {
                    println!("Data channel '{}'-'{}' open", channel.label(), channel.id());
                    let _ = channel_proxy.lock().await.handle_event(*chan_type, GssvClientEvent::ChannelOpen).await;
                })
            }))
            .await;

        let channel_clone = channel.clone();
        let channel_proxy_clone = channel_proxy.clone();
        channel
            .on_close(Box::new(move || {
                let channel_proxy = channel_proxy_clone.clone();
                let channel = Arc::clone(&channel_clone);
                Box::pin(async move {
                    println!("Data channel '{}'-'{}' close", channel.label(), channel.id());
                    let _ = channel_proxy.lock().await.handle_event(*chan_type, GssvClientEvent::ChannelClose).await;
                })
            }))
            .await;

        let chan_type_clone = chan_type.clone();
        let channel_proxy_clone = channel_proxy.clone();
        channel
            .on_message(Box::new(move |msg: DataChannelMessage| {
                let channel_proxy = channel_proxy_clone.clone();
                let msg = match String::from_utf8(msg.data.to_vec()) {
                    Ok(str) => DataChannelMsg::String(str),
                    _ => {
                        DataChannelMsg::Bytes(msg.data.to_vec())
                    }
                };
                Box::pin(async move {
                    println!("Message from DataChannel '{:?}': '{:?}'", chan_type, &msg);
                    let _ = channel_proxy.lock().await.handle_message(chan_type_clone, msg).await;
                })
            }))
            .await;

        channel_defs.lock().await.insert(chan_type.to_owned(), channel);
    }

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

    let (rumble_tx, mut rumble_rx) = mpsc::channel(10);

    // Start task that listens to mpsc receiver from ChannelProxy for messages to send out
    let chan_defs_inner = channel_defs.clone();
    let channel_recv_loop = tokio::spawn(async move {
        loop {
            let recv_res = match channel_rx.recv().await {
                Some((chan_type, msg)) => {
                    match chan_defs_inner.lock().await.get(&chan_type) {
                        Some(chan) => {
                            match msg {
                                ChannelExchangeMsg::DataChannel(bla) => {
                                    match bla {
                                        DataChannelMsg::Bytes(msg_bytes) => chan.send(&msg_bytes.into()).await,
                                        DataChannelMsg::String(msg_str) => chan.send_text(msg_str).await,
                                    }
                                },
                                ChannelExchangeMsg::ChannelEvent(evt) => {
                                    match evt {
                                        GssvChannelEvent::GamepadRumble(vibration) => {
                                            let rumble_effect: BaseEffect = vibration.into();
                                            rumble_tx.send(rumble_effect).await;
                                            Ok(0)
                                        },
                                        _ => {
                                            todo!("Event currently unhandled, event={:?}", evt);
                                        }
                                    }
                                },
                                ChannelExchangeMsg::ClientEvent(evt) => {
                                    panic!("Received Client event on client side, evt={:?}", evt)
                                }
                            }
                        },
                        None => {
                            Err(webrtc::Error::new("Channel not found".to_owned()))
                        },
                    }
                },
                None => {
                    Ok(0)
                }
            };

            if let Err(err) = recv_res {
                eprintln!("Failed to receive message from ChannelProxy, error: {:?}", err)
            }
        }
    });

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

    let appsrc_audio_arc = Arc::new(appsrc_audio);
    let appsrc_video_arc = Arc::new(appsrc_video);

    // Set a handler for when a new remote track starts, this handler saves buffers to disk as
    // an ivf file, since we could have multiple video tracks we provide a counter.
    // In your application this is where you would handle/process video
    let pc = Arc::downgrade(&peer_connection);
    peer_connection.on_track(Box::new(move |track: Option<Arc<TrackRemote>>, _receiver: Option<Arc<RTCRtpReceiver>>| {
        if let Some(track) = track {
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

            let notify_rx2 = Arc::clone(&notify_rx);
            let appsrc_audio_clone = Arc::clone(&appsrc_audio_arc);
            let appsrc_video_clone = Arc::clone(&appsrc_video_arc);
            //let h264_writer2 = Arc::clone(&h264_writer);
            //let ogg_writer2 = Arc::clone(&ogg_writer);
            Box::pin(async move {
                let codec = track.codec().await;
                let mime_type = codec.capability.mime_type.to_lowercase();

                if mime_type == MIME_TYPE_OPUS.to_lowercase() {
                    println!("Got Opus track, sending to audio appsrc");
                    tokio::spawn(async move {
                        loop {
                            let buffer = read_packet_into_gst_buf(&track).await.expect("Failed to read packet into buffer");
                            appsrc_audio_clone.push_buffer(buffer).expect("Failed to push buffer for audio");
                        }
                    });
                } else if mime_type == MIME_TYPE_H264.to_lowercase() {
                    println!("Got h264 track, sending to video appsrc");
                    tokio::spawn(async move {
                        loop {
                            let buffer = read_packet_into_gst_buf(&track).await.expect("Failed to read packet into buffer");
                            appsrc_video_clone.push_buffer(buffer).expect("Failed to push buffer for video");
                        }
                    });
                }
            })
        }else {
            Box::pin(async {})
        }
	})).await;

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
        let json = c.to_json().await?;
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

    let keepalive_loop = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            match xcloud.send_keepalive(&session).await {
                Ok(_resp) => {
                    // println!("Keepalive response: {:?}", resp);
                },
                Err(err) => {
                    eprintln!("Sending keepalive faild!, err={:?}", err);
                }
            }
        }
    });
    

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    loop {
        let mut gamepad_processor = GamepadProcessor::new();
        // Examine new events
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            println!("{:?} New event from {}: {:?}", time, id, event);
            gamepad_processor.add_event(event);
            let gamepad_data = gamepad_processor.get_data();
            channel_proxy.lock().await.handle_input(&gamepad_data).await.unwrap();

            if let Ok(rumble_effect) = rumble_rx.try_recv() {
                if gilrs.gamepad(id).is_ff_supported() {
                    EffectBuilder::new()
                    .add_effect(rumble_effect)
                    .gamepads(&[id])
                    .finish(&mut gilrs)?
                    .play()?;
                }
            }
        }
    }

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                dbg!(&msg, err);
                return Err("err".into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;



    println!("Press ctrl-c to stop");
    tokio::select! {
        _ = done_rx.recv() => {
            println!("received done signal!");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("");
        }
    };

    Ok(())
}
