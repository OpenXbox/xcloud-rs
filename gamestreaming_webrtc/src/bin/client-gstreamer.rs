use gst_webrtc::{ffi::GstWebRTCRTPTransceiver, glib, gst::StructureRef};
use gstreamer_webrtc as gst_webrtc;
use gstreamer_webrtc::gst;
use gst::{prelude::*, ElementFactory};

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

fn on_offer_created(reply: &StructureRef, webrtc: gst::Element) {
    dbg!("create-offer callback");
    dbg!("reply", reply);
    let offer = reply
        .get::<gst_webrtc::WebRTCSessionDescription>("offer")
        .expect("Invalid argument");

    let sdp_text = offer.sdp().as_text().unwrap();
    dbg!("offer {}", sdp_text);
    webrtc
        .emit_by_name::<()>("set-local-description", &[&offer, &None::<gst::Promise>]);
}

fn on_negotiation_needed(values: &[glib::Value]) {
    dbg!("on-negotiation-needed {:?}", values);
    let webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
    let clone = webrtc.clone();
    let promise = gst::Promise::with_change_func(move |res| {
        dbg!("on-offer-promise", res);
        match res {
            Ok(res) => {
                on_offer_created(res.unwrap(), clone)
            },
            Err(err) => {
                eprintln!("Promise error: {:?}", err);
            }
        }
    });
    let options = gst::Structure::new_empty("options");
    webrtc.emit_by_name::<()>("create-offer", &[&options, &promise]);
}

fn tutorial_main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // Constants
    let H264_VIDEO = gst::Caps::new_simple(
        "application/x-rtp",
        &[
            ("media", &"video"),
            ("encoding-name", &"H264"),
            ("payload", &("96")),
            ("clock-rate", &("90000")),
        ],
    );

    let OPUS_AUDIO = gst::Caps::new_simple(
        "application/x-rtp",
        &[
            ("media", &"audio"),
            ("encoding-name", &"OPUS"),
            ("payload", &("97")),
            ("clock-rate", &("48000")),
            ("encoding-params", &"2"),
        ],
    );

    // Create elements
    let webrtc = gst::ElementFactory::make("webrtcbin")
        .name("recv")
        .property("stun-server", "stun://stun.l.google.com:19302")
        .build()
        .expect("Failed to create webrtcbin");
    
    let depay = gst::ElementFactory::make("rtpvp8depay")
        .build()
        .expect("Failed to create depay");

    let decoder = gst::ElementFactory::make("vp8dec")
        .build()
        .expect("Failed to create decoder");

    let convert = gst::ElementFactory::make("videoconvert")
        .build()
        .expect("Failed to create convert");

    let queue = gst::ElementFactory::make("queue")
        .build()
        .expect("Failed to create queue");

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
            &queue,
            &sink,
        ])
        .expect("Failed to add to pipeline");

    gst::Element::link_many(&[ &depay, &decoder, &convert, &queue, &sink])
        .expect("Failed to link elements");

    // Connect callbacks
    webrtc.connect("on-negotiation-needed", false, move |values| {
        on_negotiation_needed(values);
        None
    });
    webrtc.connect("on-ice-candidate", false, |x| { eprintln!("On ICE candidate"); None });
    webrtc.connect("pad-added", false, |x| { eprintln!("Pad added"); None });
    
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

    // Start playing
    pipeline
        .set_state(gst::State::Playing)
        .expect("Failed setting PLAYING state");

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
            _ => (),
        }
    }

    // Shutdown pipeline
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

fn main() {
    // tutorials_common::run is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    run(tutorial_main);
}

