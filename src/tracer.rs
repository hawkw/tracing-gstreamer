use std::cell::RefCell;

use glib::subclass::basic;
use gstreamer::{
    glib,
    prelude::PadExtManual,
    subclass::prelude::*,
    traits::{GstObjectExt, PadExt},
    Buffer, FlowReturn, Object, Pad, Query, Tracer,
};
use tracing::{
    span::{EnteredSpan, Span},
    Callsite,
};
use tracing_core::Kind;

pub struct TracingTracerPriv {
    _p: (),
}

thread_local! {
    static SPAN_STACK: RefCell<Vec<EnteredSpan>> = RefCell::new(Vec::new());
}

impl TracingTracerPriv {
    fn push_span(&self, span: EnteredSpan) {
        SPAN_STACK.with(|stack| {
            stack.borrow_mut().push(span);
        })
    }
    fn pop_span(&self) -> Option<EnteredSpan> {
        SPAN_STACK.with(|stack| stack.borrow_mut().pop())
    }

    fn post(&self) {
        if let Some(span) = self.pop_span() {
            drop(span);
        }
    }

    fn pad_pre(&self, name: &'static str, pad: &Pad) {
        let callsite = crate::callsite::DynamicCallsites::get().callsite_for(
            tracing::Level::ERROR,
            name,
            name,
            None,
            None,
            None,
            Kind::SPAN,
            &["gstpad.state", "gstpad.parent.name"],
        );
        let interest = callsite.interest();
        if interest.is_never() {
            return;
        }

        let meta = callsite.metadata();
        tracing_core::dispatcher::get_default(move |dispatch| {
            if !dispatch.enabled(meta) {
                return;
            }
            let gstpad_flags_value = Some(tracing_core::field::display(crate::PadFlags(
                pad.pad_flags().bits(),
            )));
            let gstpad_parent = pad.parent_element();
            let gstpad_parent_name_value = gstpad_parent.map(|p| p.name());
            let gstpad_parent_name_value = gstpad_parent_name_value.as_ref().map(|n| n.as_str());
            let fields = meta.fields();
            let mut fields_iter = fields.into_iter();
            let values = field_values![fields_iter =>
                // /!\ /!\ /!\ Must be in the same order as the field list above /!\ /!\ /!\
                "gstpad.flags" = gstpad_flags_value;
                "gstpad.parent.name" = gstpad_parent_name_value;
            ];
            let valueset = fields.value_set(&values);
            let span = Span::new_root_with(meta, &valueset, dispatch);
            if span.is_disabled() {
                return;
            }
            let entered = span.entered();
            self.push_span(entered);
        });
    }
}

glib::wrapper! {
    pub struct TracingTracer(ObjectSubclass<TracingTracerPriv>)
       @extends Tracer, Object;
}

#[glib::object_subclass]
impl ObjectSubclass for TracingTracerPriv {
    const NAME: &'static str = "TracingTracer";
    type Type = TracingTracer;
    type Class = basic::ClassStruct<Self>;
    type Instance = basic::InstanceStruct<Self>;
    type ParentType = Tracer;
    type Interfaces = ();

    fn new() -> Self {
        Self { _p: () }
    }
}

impl ObjectImpl for TracingTracerPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);
        self.register_hook(TracerHook::PadPushPost);
        self.register_hook(TracerHook::PadPushPre);
        self.register_hook(TracerHook::PadPushListPost);
        self.register_hook(TracerHook::PadPushListPre);
        self.register_hook(TracerHook::PadQueryPost);
        self.register_hook(TracerHook::PadQueryPre);
        self.register_hook(TracerHook::PadPushEventPost);
        self.register_hook(TracerHook::PadPushEventPre);
        self.register_hook(TracerHook::PadPullRangePost);
        self.register_hook(TracerHook::PadPullRangePre);
    }
}

impl TracerImpl for TracingTracerPriv {
    fn pad_push_pre(&self, _: u64, pad: &Pad, _: &Buffer) {
        self.pad_pre("pad_push", pad);
    }

    fn pad_push_list_pre(&self, _: u64, pad: &Pad, _: &gstreamer::BufferList) {
        self.pad_pre("pad_push_list", pad);
    }

    fn pad_query_pre(&self, _: u64, pad: &Pad, _: &Query) {
        self.pad_pre("pad_query", pad);
    }

    fn pad_push_event_pre(&self, _: u64, pad: &Pad, _: &gstreamer::Event) {
        self.pad_pre("pad_event", pad);
    }

    fn pad_pull_range_pre(&self, _: u64, pad: &Pad, _: u64, _: u32) {
        self.pad_pre("pad_pull_range", pad);
    }

    fn pad_pull_range_post(&self, _: u64, _: &Pad, _: &Buffer, _: FlowReturn) {
        self.post()
    }

    fn pad_push_event_post(&self, _: u64, _: &Pad, _: bool) {
        self.post()
    }

    fn pad_push_list_post(&self, _: u64, _: &Pad, _: FlowReturn) {
        self.post()
    }

    fn pad_push_post(&self, _: u64, _: &Pad, _: FlowReturn) {
        self.post()
    }

    fn pad_query_post(&self, _: u64, _: &Pad, _: &Query, _: bool) {
        self.post()
    }
}
