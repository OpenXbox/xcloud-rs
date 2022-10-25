use std::{fs::File, io::Write, str::FromStr, sync::Mutex};

use gamestreaming_webrtc::{GamestreamingClient, Platform, api::{SessionResponse, IceCandidate}};
use gst_webrtc::{ffi::{GstWebRTCRTPTransceiver, GstWebRTCDataChannel, GstWebRTCBundlePolicy}, glib, gst::StructureRef, WebRTCSessionDescription, gst_sdp::SDPMessage, WebRTCBundlePolicy, WebRTCICETransportPolicy};
use gstreamer_webrtc as gst_webrtc;
use gstreamer_webrtc::gst;
use gst::{prelude::*, ElementFactory};
use xal::utils::TokenStore;

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

fn on_offer_created(reply: &StructureRef, webrtc: gst::Element, xcloud: GamestreamingClient, session: &SessionResponse) {
    println!("create-offer callback");
    
    let offer = reply
        .get::<gst_webrtc::WebRTCSessionDescription>("offer")
        .expect("Invalid argument");

    let sdp_text = offer.sdp().as_text().expect("Failed to get SDP text");
    dbg!(&sdp_text);


    webrtc
        .emit_by_name::<()>("set-local-description", &[&offer, &None::<gst::Promise>]);


    let sdp_response = xcloud.exchange_sdp(session, &sdp_text)
        .expect("exchange sdp failed");
    dbg!(&sdp_response);
    let sdp_response_text = sdp_response.exchange_response.sdp
        .expect("Failed unrwapping SDP section");
    let ret = SDPMessage::parse_buffer(sdp_response_text.as_bytes())
        .expect("Failed parsing SDP");
    let answer = 
        gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);
    webrtc
        .emit_by_name::<()>("set-remote-description", &[&answer, &None::<gst::Promise>]);
}

fn send_ice_candidate_message(
    values: &[glib::Value],
    candidates: &mut Box<Vec<IceCandidate>>,
    xcloud: &GamestreamingClient,
    session: &SessionResponse,
    webrtc: &gst::Element,
) {
    //dbg!(values);
    let mlineindex = values[1].get::<u32>().expect("Invalid argument");
    let candidate = values[2].get::<String>().expect("Invalid argument");

    dbg!("Adding ICE candidate to pending list", &values);
    candidates.push(IceCandidate {
        candidate: candidate,
        sdp_mid: None,
        sdp_mline_index: Some(mlineindex as u16),
        username_fragment: None,
    });

    dbg!("all", &candidates);
    if candidates.len() == 6 {
        eprintln!("Sending over ICE candidates");
        let bla = candidates.clone();
        let result = xcloud.exchange_ice(session, *bla)
            .expect("Failed ICE exchange");
        eprintln!("Adding remote ICE candidates");
        for candidate in result.exchange_response {
            
            let c = candidate.candidate;
            let sdmlineindex = candidate.sdp_mline_index.unwrap() as u32;
            eprintln!("Adding remote ICE candidate: {:?} :::::::: {:?}", &c, sdmlineindex);

            webrtc
                .emit_by_name::<()>("add-ice-candidate", &[&sdmlineindex, &c]);
        }
    }
    
}

fn on_negotiation_needed(values: &[glib::Value], xcloud: &GamestreamingClient, session: &SessionResponse) {
    println!("on-negotiation-needed");
    let webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
    let clone = webrtc.clone();
    let xcloud_clone = xcloud.clone();
    let session_clone = session.clone();
    let promise = gst::Promise::with_change_func(move |res| {
        match res {
            Ok(res) => {
                on_offer_created(res.unwrap(), clone, xcloud_clone, &session_clone);
            },
            Err(err) => {
                eprintln!("Promise error: {:?}", err);
            }
        }
    });
    let options = gst::Structure::new_empty("options");
    webrtc.emit_by_name::<()>("create-offer", &[&options, &promise]);
}

const TOKENS_FILEPATH: &'static str = "tokens.json";

fn gstreamer_main() {
    let ts = match TokenStore::load(TOKENS_FILEPATH) {
        Ok(ts) => ts,
        Err(err) => {
            eprintln!("Failed to load tokens!");
            return;
        }
    };

    let xcloud = GamestreamingClient::create(
        Platform::Cloud,
    &ts.gssv_token.token_data.token,
    &ts.xcloud_transfer_token.lpt).unwrap();

    let session = match xcloud.lookup_games().unwrap().first() {
        Some(title) => {
            println!("Starting title: {:?}", title);
            let session = xcloud.start_stream_xcloud(&title.title_id).unwrap();
            println!("Session started successfully: {:?}", session);

            session
        }
        None => {
            eprintln!("No titles received from API");
            return;
        }
    };

    // Initialize GStreamer
    gst::init().unwrap();

    // Constants
    let H264_VIDEO = gst::Caps::from_str("application/x-rtp, media= (string)video, clock-rate= (int)90000, encoding-name= (string)H264, payload= (int)96").unwrap();
    let OPUS_AUDIO = gst::Caps::from_str("application/x-rtp, media= (string)audio, clock-rate= (int)48000, encoding-name= (string)OPUS, payload= (int)97").unwrap();

    // Create elements
    let webrtc = gst::ElementFactory::make("webrtcbin")
        .name("recv")
        .property("stun-server", "stun://stun.l.google.com:19302")
        .property("bundle-policy", WebRTCBundlePolicy::MaxBundle) // 0: none, 1: balanced, 2 max-compat, 3: max-bundle
        .property("async-handling", true)
        //.property("ice-transport-policy", WebRTCICETransportPolicy::All) // 0: all, 1: relay
        //.property("message-forward", true)
        .build()
        .expect("Failed to create webrtcbin");
    
    let depay = gst::ElementFactory::make("rtph264depay")
        .build()
        .expect("Failed to create depay");

    let decoder = gst::ElementFactory::make("avdec_h264")
        .build()
        .expect("Failed to create decoder");

    let convert = gst::ElementFactory::make("videoconvert")
        .build()
        .expect("Failed to create convert");

    let sink = gst::ElementFactory::make("autovideosink")
        .build()
        .expect("Failed to create sink");

    // Build the pipeline
    let pipeline = gst::Pipeline::builder().name("test-pipeline").build();

    pipeline
        .add_many(&[
            &webrtc,
            &depay,
            &decoder,
            &convert,
            &sink,
        ])
        .expect("Failed to add to pipeline");

    gst::Element::link_many(&[ &depay, &decoder, &convert, &sink])
        .expect("Failed to link elements");

    // Connect callbacks
    let xcloud_clone = xcloud.clone();
    let xcloud_clone2 = xcloud.clone();
    let session_clone = session.clone();
    let session_clone2 = session.clone();
    let mut candidates: Vec<IceCandidate> = vec![];
    let cs_box  = Mutex::new(Box::new(candidates));
    let webrtc_clone = Box::new(webrtc.clone());
    webrtc.connect("on-negotiation-needed", false, move |values| {
        on_negotiation_needed(values, &xcloud_clone, &session_clone);
        None
    });
    webrtc.connect("on-ice-candidate", false, move |values| {
        let mut cs_box_clone = cs_box.lock().expect("Failed mutex lock");
        send_ice_candidate_message( values, &mut cs_box_clone, &xcloud_clone2, &session_clone2, &webrtc_clone);
        None
    });

    // Create transceivers
    // Video: Recvonly / H264
    // Audio: SenvRecv / Opus
    webrtc
        .emit_by_name::<glib::Object>(
            "add-transceiver",
            &[
                &gst_webrtc::WebRTCRTPTransceiverDirection::Recvonly,
                &H264_VIDEO,
            ],
        );

    webrtc
        .emit_by_name::<glib::Object>(
            "add-transceiver",
            &[
                &gst_webrtc::WebRTCRTPTransceiverDirection::Sendrecv,
                &OPUS_AUDIO,
            ],
        );

    println!("Channels created");
    // dbg!(input_channel, control_channel, message_channel, chat_channel);

    // Start playing
    pipeline
        .set_state(gst::State::Playing)
        .expect("Failed setting PLAYING state");

    // Create datachannels
    // INPUT, protocol: "1.0", ordered: true
    let input_init_struct = gst::Structure::builder("options")
        .field("ordered", true)
        .field("protocol", "1.0")
        .field("id", 3)
        .build();

    let input_channel = webrtc
        .emit_by_name_with_values("create-data-channel",&["input".to_value(), input_init_struct.to_value()]);

    // CONTROL, protocol: "controlV1"
    let control_init_struct = gst::Structure::builder("options")
        .field("protocol", "controlV1")
        .field("id", 4)
        .build();
    let control_channel = webrtc
        .emit_by_name_with_values("create-data-channel",&["control".to_value(), control_init_struct.to_value()]);

    // MESSAGE, protocol: "messageV1"
    let message_init_struct = gst::Structure::builder("options")
        .field("protocol", "messageV1")
        .field("id", 5)
        .build();
    let message_channel = webrtc
        .emit_by_name_with_values("create-data-channel",&["message".to_value(), message_init_struct.to_value()]);

    // CHAT, protocol: "chatV1"
    let chat_init_struct = gst::Structure::builder("options")
        .field("protocol", "chatV1")
        .field("id", 6)
        .build();
    let chat_channel = webrtc
        .emit_by_name_with_values("create-data-channel",&["chat".to_value(), chat_init_struct.to_value()]);


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
            val => {
                dbg!(val);
            },
        }
    }

    // Shutdown pipeline
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

fn main() {
    // run wrapper is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    run(gstreamer_main);
}

