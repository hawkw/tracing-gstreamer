//! This example prepares a vp9 encoding pipeline, instrumented via tracing.
use gstreamer::{traits::ElementExt, ClockTime, MessageView::*, State};

fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_thread_names(true)
        .with_thread_ids(true)
        .init();
    tracing_gstreamer::integrate_events();
    gstreamer::debug_remove_default_log_function();
    gstreamer::debug_set_default_threshold(gstreamer::DebugLevel::Count);
    gstreamer::init().expect("gst init");
    tracing_gstreamer::integrate_spans();

    let pipeline = gstreamer::parse_launch(
        r#"
        videotestsrc num-buffers=120
        ! vp9enc
        ! webmmux name=mux
        ! fakesink sync=false

        audiotestsrc num-buffers=120
        ! opusenc
        ! mux.
    "#,
    )
    .expect("construct the pipeline");
    let bus = pipeline.bus().expect("could not obtain the pipeline bus");
    pipeline
        .set_state(gstreamer::State::Playing)
        .expect("could not start the pipeline");
    loop {
        let msg = match bus.timed_pop(ClockTime::NONE) {
            None => break,
            Some(msg) => msg,
        };
        tracing::info!(message = "bus message", bus_message = ?msg);
        match msg.view() {
            Eos(_) => break,
            Error(e) => break tracing::error!("{}", e.error()),
            Warning(w) => tracing::warn!("{:?}", w),
            _ => {}
        }
    }
    pipeline
        .set_state(State::Null)
        .expect("could not stop the pipeline");
}
