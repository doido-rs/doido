//! Custom `tracing_subscriber` formatters that enrich ERROR events with a Rust
//! backtrace and a tracing span stack (captured by [`tracing_error::ErrorLayer`]).

use std::backtrace::{Backtrace, BacktraceStatus};
use std::fmt;
use tracing::Event;
use tracing_error::{SpanTrace, SpanTraceStatus};
use tracing_subscriber::fmt::format::{Format, FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::FmtContext;

/// Wraps the stock compact/pretty formatter and appends diagnostic context
/// after every ERROR event.
pub struct ErrorContextFormat {
    pretty: bool,
}

impl ErrorContextFormat {
    pub fn compact() -> Self {
        Self { pretty: false }
    }

    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl<S, N> FormatEvent<S, N> for ErrorContextFormat
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if self.pretty {
            Format::default().pretty().format_event(ctx, writer.by_ref(), event)?;
        } else {
            Format::default().format_event(ctx, writer.by_ref(), event)?;
        }

        if event.metadata().level() == &tracing::Level::ERROR {
            write_error_context(writer.by_ref())?;
        }
        Ok(())
    }
}

fn write_error_context(mut writer: Writer<'_>) -> fmt::Result {
    let backtrace = Backtrace::force_capture();
    if backtrace.status() == BacktraceStatus::Captured {
        writeln!(writer, "backtrace:")?;
        writeln!(writer, "{backtrace}")?;
    }

    let span_trace = SpanTrace::capture();
    if span_trace.status() == SpanTraceStatus::CAPTURED {
        writeln!(writer, "span trace:")?;
        writeln!(writer, "{span_trace}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_error_context_formats_backtrace_and_span_trace() {
        let mut buf = String::new();
        write_error_context(Writer::new(&mut buf)).unwrap();
        assert!(buf.contains("backtrace:"));
    }
}
