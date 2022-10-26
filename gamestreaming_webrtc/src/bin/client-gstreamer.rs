use gamestreaming_webrtc::{
    api::{IceCandidate, SessionResponse},
    GamestreamingClient, Platform,
};
use gst::prelude::*;
use gstreamer_webrtc::{self as gst_webrtc, gst};
use std::{str::FromStr, sync::Mutex};
use xal::utils::TokenStore;

use anyhow::{Context, Result, anyhow};
use derive_more::{Display, Error};

const H264_VIDEO_CAPS: &'static str = "application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, payload=96, packetization-mode=(string)1, profile-level-id=(string)42c016";
const OPUS_AUDIO_CAPS: &'static str =
    "application/x-rtp, media=audio, clock-rate=48000, encoding-name=OPUS, payload=97";

/// macOS has a specific requirement that there must be a run loop running on the main thread in
/// order to open windows and use OpenGL, and that the global NSApplication instance must be
/// initialized.

/// On macOS this launches the callback function on a thread.
/// On other platforms it's just executed immediately.
#[cfg(not(target_os = "macos"))]
pub fn run<T, F: FnOnce() -> T + Send + 'static>(main: F) -> T
where
    T: Send + 'static,
{
    main()
}

#[cfg(target_os = "macos")]
pub fn run<T, F: FnOnce() -> T + Send + 'static>(main: F) -> T
where
    T: Send + 'static,
{
    use cocoa::appkit::NSApplication;

    use std::thread;

    unsafe {
        let app = cocoa::appkit::NSApp();
        let t = thread::spawn(|| {
            let res = main();

            let app = cocoa::appkit::NSApp();
            app.stop_(cocoa::base::nil);

            // Stopping the event loop requires an actual event
            let event = cocoa::appkit::NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(
                cocoa::base::nil,
                cocoa::appkit::NSEventType::NSApplicationDefined,
                cocoa::foundation::NSPoint { x: 0.0, y: 0.0 },
                cocoa::appkit::NSEventModifierFlags::empty(),
                0.0,
                0,
                cocoa::base::nil,
                cocoa::appkit::NSEventSubtype::NSApplicationActivatedEventType,
                0,
                0,
            );
            app.postEvent_atStart_(event, cocoa::base::YES);

            std::process::exit(0);

            res
        });

        app.run();

        t.join().unwrap()
    }
}

fn on_offer_created(
    reply: &gst::StructureRef,
    webrtc: gst::Element,
    xcloud: GamestreamingClient,
    session: &SessionResponse,
) -> anyhow::Result<()> {
    println!("create-offer callback");

    let offer: gst_webrtc::WebRTCSessionDescription = reply.get("offer")?;

    let sdp_text = offer.sdp().as_text()?;
    eprintln!("Local offer: {:?}", &sdp_text);

    println!("Setting local description");
    webrtc.emit_by_name::<()>("set-local-description", &[&offer, &None::<gst::Promise>]);

    let sdp_response = xcloud.exchange_sdp(session, &sdp_text)?;
    eprintln!("Remote answer: {:?}", &sdp_response);
    let sdp_response_text = sdp_response.exchange_response.sdp.unwrap();
    let ret = gst_webrtc::gst_sdp::SDPMessage::parse_buffer(sdp_response_text.as_bytes())?;
    let answer = gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);

    println!("Setting remote description");
    webrtc.emit_by_name::<()>("set-remote-description", &[&answer, &None::<gst::Promise>]);

    Ok(())
}

fn send_ice_candidate_message(
    values: &[gst::glib::Value],
    candidates: &mut Box<Vec<IceCandidate>>,
    xcloud: &GamestreamingClient,
    session: &SessionResponse,
    webrtc: &gst::Element,
) -> anyhow::Result<()> {
    ////dbg!(values);
    let mlineindex = values[1].get::<u32>()?;
    let candidate = values[2].get::<String>()?;

    //dbg!("Adding ICE candidate to pending list", &values);
    candidates.push(IceCandidate {
        candidate: candidate,
        sdp_mid: None,
        sdp_mline_index: Some(mlineindex as u16),
        username_fragment: None,
    });

    //dbg!("all", &candidates);
    if candidates.len() == 6 {
        eprintln!("Sending over ICE candidates");
        let bla = candidates.clone();
        let result = xcloud
            .exchange_ice(session, *bla)
            .expect("Failed ICE exchange");
        eprintln!("Adding remote ICE candidates");
        for candidate in result.exchange_response {
            // Trimming candidate string to remove whitespace at the end
            let c = candidate.candidate.trim();
            let sdmlineindex = candidate.sdp_mline_index.unwrap() as u32;
            eprintln!(
                "Adding remote ICE candidate: {:?} :::::::: {:?}",
                &c, sdmlineindex
            );

            webrtc.emit_by_name::<()>("add-ice-candidate", &[&sdmlineindex, &c]);
        }
    }
    Ok(())
}

fn on_negotiation_needed(
    values: &[gst::glib::Value],
    xcloud: &GamestreamingClient,
    session: &SessionResponse,
) -> anyhow::Result<()> {
    println!("on-negotiation-needed");
    let webrtc = values[0].get::<gst::Element>()?;
    let clone = webrtc.clone();
    let xcloud_clone = xcloud.clone();
    let session_clone = session.clone();
    let promise = gst::Promise::with_change_func(move |res| match res {
        Ok(res) => match res {
            Some(offer_res) => {
                let _ = on_offer_created(offer_res, clone, xcloud_clone, &session_clone);
            }
            None => {}
        },
        Err(err) => {
            eprintln!("Promise error: {:?}", err);
        }
    });
    let options = gst::Structure::new_empty("options");
    webrtc.emit_by_name::<()>("create-offer", &[&options, &promise]);

    Ok(())
}

const TOKENS_FILEPATH: &'static str = "tokens.json";

fn create_datachannels(
    webrtc: &gst::Element,
) -> anyhow::Result<
    (
        Option<gst_webrtc::glib::Value>,
        Option<gst_webrtc::glib::Value>,
        Option<gst_webrtc::glib::Value>,
        Option<gst_webrtc::glib::Value>,
    )
> {
    // Create datachannels
    // INPUT, protocol: "1.0", ordered: true
    let input_init_struct = gst::Structure::builder("options")
        .field("ordered", true)
        .field("protocol", "1.0")
        .field("id", 3)
        .build();

    let input_channel = webrtc.emit_by_name_with_values(
        "create-data-channel",
        &["input".to_value(), input_init_struct.to_value()],
    );

    // CONTROL, protocol: "controlV1"
    let control_init_struct = gst::Structure::builder("options")
        .field("protocol", "controlV1")
        .field("id", 4)
        .build();
    let control_channel = webrtc.emit_by_name_with_values(
        "create-data-channel",
        &["control".to_value(), control_init_struct.to_value()],
    );

    // MESSAGE, protocol: "messageV1"
    let message_init_struct = gst::Structure::builder("options")
        .field("protocol", "messageV1")
        .field("id", 5)
        .build();
    let message_channel = webrtc.emit_by_name_with_values(
        "create-data-channel",
        &["message".to_value(), message_init_struct.to_value()],
    );

    // CHAT, protocol: "chatV1"
    let chat_init_struct = gst::Structure::builder("options")
        .field("protocol", "chatV1")
        .field("id", 6)
        .build();
    let chat_channel = webrtc.emit_by_name_with_values(
        "create-data-channel",
        &["chat".to_value(), chat_init_struct.to_value()],
    );

    Ok((
        input_channel,
        control_channel,
        message_channel,
        chat_channel,
    ))
}

fn gstreamer_main() -> anyhow::Result<()> {
    let ts = TokenStore::load(TOKENS_FILEPATH)
        .map_err(|_|anyhow!("Failed_to_load_tokens"))?;

    let xcloud = GamestreamingClient::create(
        Platform::Cloud,
        &ts.gssv_token.token_data.token,
        &ts.xcloud_transfer_token.lpt,
    )
    .context("Failed to create gamestreaming client")?;

    let session = match xcloud.lookup_games().context("Failed looking up games")?.get(2) {
        Some(title) => {
            println!("Starting title: {:?}", title);
            let session = xcloud.start_stream_xcloud(&title.title_id)
                .context("Failed starting stream")?;
            println!("Session started successfully: {:?}", session);

            session
        }
        None => {
            return Err(anyhow::anyhow!("No titles received from API"));
        }
    };

    // Initialize GStreamer
    gst::init().unwrap();

    // Create elements
    let webrtc = gst::ElementFactory::make("webrtcbin")
        .name("recv")
        .property("stun-server", "stun://stun.l.google.com:19302")
        .property("bundle-policy", gst_webrtc::WebRTCBundlePolicy::MaxBundle)
        .build()
        .context("Failed to create webrtcbin")?;

    // VIDEO
    let video_jitterbuffer = gst::ElementFactory::make("rtpjitterbuffer").build()?;
    let video_depay = gst::ElementFactory::make("rtph264depay").build()?;
    let video_decoder = gst::ElementFactory::make("avdec_h264").build()?;
    let video_convert = gst::ElementFactory::make("videoconvert").build()?;
    let video_sink = gst::ElementFactory::make("autovideosink")
        .property("async-handling", true)
        .property("sync", false)
        .build()
        .context("Failed to create video_sink")?;

    // AUDIO
    let audio_depay = gst::ElementFactory::make("rtpopusdepay").build()?;
    let audio_decoder = gst::ElementFactory::make("opusdec").build()?;
    let audio_convert = gst::ElementFactory::make("audioconvert").build()?;
    let audio_queue = gst::ElementFactory::make("queue").build()?;
    let audio_sink = gst::ElementFactory::make("pipewiresink").build()?;

    // Build the pipeline
    let pipeline = gst::Pipeline::builder().name("test-pipeline").build();

    pipeline
        .add_many(&[
            &webrtc,
            &video_jitterbuffer,
            &video_depay,
            &video_decoder,
            &video_convert,
            &video_sink,
            &audio_depay,
            &audio_decoder,
            &audio_convert,
            &audio_queue,
            &audio_sink,
        ])?;
    gst::Element::link_many(&[
        &video_jitterbuffer,
        &video_depay,
        &video_decoder,
        &video_convert,
        &video_sink,
    ])?;
    gst::Element::link_many(&[
        &audio_depay,
        &audio_decoder,
        &audio_convert,
        &audio_queue,
        &audio_sink,
    ])?;

    // Connect callbacks
    let xcloud_clone = xcloud.clone();
    let xcloud_clone2 = xcloud.clone();
    let session_clone = session.clone();
    let session_clone2 = session.clone();
    let candidates: Vec<IceCandidate> = vec![];
    let cs_box = Mutex::new(Box::new(candidates));
    let webrtc_clone = Box::new(webrtc.clone());
    webrtc.connect("on-negotiation-needed", false, move |values| {
        if let Err(err) = on_negotiation_needed(values, &xcloud_clone, &session_clone) {
            eprintln!("Handling on-negotiation-needed failed: {}", err)
        }
        None
    });
    webrtc.connect("on-ice-candidate", false, move |values| {
        let mut cs_box_clone = cs_box.lock().expect("Failed mutex lock");
        if let Err(err) = send_ice_candidate_message(
            values,
            &mut cs_box_clone,
            &xcloud_clone2,
            &session_clone2,
            &webrtc_clone,
        ) {
            eprintln!("Handling ICE candidate message failed: {}", err)
        }
        None
    });
    /*
    webrtc.connect("on-data-channel", false, move |values| {
        None
    });
     */

    webrtc.connect_pad_added(move |_, pad| {
        let pad_name = pad.name();
        eprintln!("Pad added {} {:?}", pad_name, pad.direction());
        if pad_name == "src_0" {
            dbg!(pad.caps());
            println!("Video Pad: {:?}", pad_name);

            let depay_sink = &video_jitterbuffer
                .static_pad("sink")
                .expect("Failed to get sink from video_depay");
            pad.link(depay_sink)
                .expect("Failed to link video src to depay_sink");
        } else if pad_name == "src_1" {
            println!("Audio Pad: {:?}", pad_name);
            let depay_sink = &audio_depay
                .static_pad("sink")
                .expect("Failed to get sink from audio_depay");
            pad.link(depay_sink)
                .expect("Failed to link audio src to depay_sink");
        } else {
            //unreachable!()
        };
    });

    // Create transceivers
    // Video: Recvonly / H264
    // Audio: SenvRecv / Opus
    webrtc.emit_by_name::<gst::glib::Object>(
        "add-transceiver",
        &[
            &gst_webrtc::WebRTCRTPTransceiverDirection::Recvonly,
            &gst::Caps::from_str(H264_VIDEO_CAPS).expect("Failed to construct H264 Caps"),
        ],
    );

    webrtc.emit_by_name::<gst::glib::Object>(
        "add-transceiver",
        &[
            &gst_webrtc::WebRTCRTPTransceiverDirection::Sendrecv,
            &gst::Caps::from_str(OPUS_AUDIO_CAPS).expect("Failed to construct OPUS Caps"),
        ],
    );

    // Start playing
    pipeline
        .set_state(gst::State::Playing)
        .expect("Failed setting PLAYING state");

    println!("Transceivers created");
    let channels = create_datachannels(&webrtc).expect("Failed to create datachannels");

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            MessageView::StateChanged(state) => {
                println!("State change: {:?}", state);
            }
            _ => {}
        }
    }

    // Shutdown pipeline
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
    Ok(())
}

fn main() {
    // run wrapper is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    match run(gstreamer_main) {
        Ok(r) => r,
        Err(e) => eprintln!("Error! {}", e),
    }
}
