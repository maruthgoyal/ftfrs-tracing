use std::time::Instant;

use tracing_core::Subscriber;
use tracing_subscriber::{fmt::MakeWriter, registry::LookupSpan, Layer};

#[derive(Debug)]
pub struct FtfLayer<W: for<'a> MakeWriter<'a>> {
    writer: W,
    start: Instant,
}

impl<W: for<'a> MakeWriter<'a>> FtfLayer<W> {
    pub fn new(writer: W) -> Self {
        let start = Instant::now();
        let magic = ftfrs::Record::create_magic_number();
        magic.write(&mut writer.make_writer()).unwrap();
        ftfrs::Record::create_provider_info(1, "test".to_string())
            .write(&mut writer.make_writer())
            .unwrap();
        Self { writer, start }
    }
}

impl<W, S: Subscriber> Layer<S> for FtfLayer<W>
where
    S: for<'a> LookupSpan<'a>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(
        &self,
        _attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        let event_start = ftfrs::Record::create_duration_begin_event(
            self.start.elapsed().as_nanos() as u64,
            ftfrs::ThreadRef::Inline{ process_koid: 1, thread_koid: 1},
            ftfrs::StringRef::Inline("trace".to_string()),
            ftfrs::StringRef::Inline(span.name().to_string()),
            Vec::new(),
        );
        event_start.write(&mut self.writer.make_writer()).unwrap();
    }

    fn on_close(
        &self,
        id: tracing_core::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(&id).unwrap();
        let event_end = ftfrs::Record::create_duration_end_event(
            self.start.elapsed().as_nanos() as u64,
            ftfrs::ThreadRef::Inline{ process_koid: 1, thread_koid: 1},
            ftfrs::StringRef::Inline("trace".to_string()),
            ftfrs::StringRef::Inline(span.name().to_string()),
            Vec::new(),
        );
        event_end.write(&mut self.writer.make_writer()).unwrap();
    }
}
