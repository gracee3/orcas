use std::borrow::Cow;
use std::env;
use std::fs::OpenOptions;
use std::path::Path;

use tracing::Event;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::{FormatEvent, Writer};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::TryInitError;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::OrcasResult;

pub fn init_file_logger(component: &str, log_path: &Path) -> OrcasResult<()> {
    let logs_parent = log_path.parent().ok_or_else(|| {
        crate::OrcasError::Transport(format!(
            "log path `{}` has no parent directory",
            log_path.display()
        ))
    })?;
    std::fs::create_dir_all(logs_parent)?;

    let aggregate_log_path = logs_parent.join("orcas.log");
    let aggregate_enabled = aggregate_enabled(component);
    let component_label = component_label(component);

    let component_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{component}=debug,debug,tokio=info")));

    let subscriber = tracing_subscriber::registry().with(
        fmt::layer()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_thread_names(false)
            .with_thread_ids(false)
            .with_writer(component_file)
            .with_ansi(false)
            .with_filter(env_filter.clone()),
    );

    if aggregate_enabled {
        let aggregate_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&aggregate_log_path)?;
        subscriber
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_file(false)
                    .with_line_number(false)
                    .with_thread_names(false)
                    .with_thread_ids(false)
                    .with_writer(aggregate_file)
                    .event_format(ComponentAggregateFormat::new(component_label))
                    .with_ansi(false)
                    .with_filter(env_filter),
            )
            .try_init()
            .map_err(|error: TryInitError| crate::OrcasError::Transport(error.to_string()))?;
    } else {
        subscriber
            .try_init()
            .map_err(|error: TryInitError| crate::OrcasError::Transport(error.to_string()))?;
    }

    Ok(())
}

pub fn runtime_cycle_enabled() -> bool {
    parse_boolish(env::var("ORCAS_LOG_RUNTIME_CYCLE").ok().as_deref(), false)
}

pub fn aggregate_enabled(component: &str) -> bool {
    match component_label(component).as_ref() {
        "tui" => parse_boolish(env::var("ORCAS_LOG_AGGREGATE_TUI").ok().as_deref(), true),
        "daemon" => parse_boolish(env::var("ORCAS_LOG_AGGREGATE_DAEMON").ok().as_deref(), true),
        "supervisor" => parse_boolish(
            env::var("ORCAS_LOG_AGGREGATE_SUPERVISOR").ok().as_deref(),
            true,
        ),
        "app-server" => parse_boolish(
            env::var("ORCAS_LOG_AGGREGATE_APP_SERVER").ok().as_deref(),
            false,
        ),
        _ => true,
    }
}

pub fn component_label(component: &str) -> Cow<'static, str> {
    match component {
        "orcas-tui" | "tui" => Cow::Borrowed("tui"),
        "orcasd" | "orcas-daemon" | "daemon" => Cow::Borrowed("daemon"),
        "orcas" | "orcas-supervisor" | "supervisor" => Cow::Borrowed("supervisor"),
        "app-server" | "codex-app-server" => Cow::Borrowed("app-server"),
        other if other.contains("tui") => Cow::Borrowed("tui"),
        other if other.contains("daemon") => Cow::Borrowed("daemon"),
        other if other.contains("supervisor") => Cow::Borrowed("supervisor"),
        other if other.contains("app-server") => Cow::Borrowed("app-server"),
        other => Cow::Owned(other.to_string()),
    }
}

fn parse_boolish(value: Option<&str>, default: bool) -> bool {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
        Some(_) => default,
        None => default,
    }
}

struct ComponentAggregateFormat {
    timer: SystemTime,
    component_label: Cow<'static, str>,
}

impl ComponentAggregateFormat {
    fn new(component_label: Cow<'static, str>) -> Self {
        Self {
            timer: SystemTime,
            component_label,
        }
    }
}

impl<S, N> FormatEvent<S, N> for ComponentAggregateFormat
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> tracing_subscriber::fmt::format::FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        self.timer.format_time(&mut writer)?;
        writer.write_char(' ')?;
        write!(
            writer,
            "{} {} ",
            event.metadata().level(),
            self.component_label
        )?;
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing::info;
    use tracing_subscriber::fmt::writer::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;

    #[derive(Clone, Default)]
    struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct BufferMakeWriter(Arc<Mutex<Vec<u8>>>);

    impl<'a> MakeWriter<'a> for BufferMakeWriter {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(Arc::clone(&self.0))
        }
    }

    #[test]
    fn component_labels_are_normalized() {
        assert_eq!(component_label("orcas-tui").as_ref(), "tui");
        assert_eq!(component_label("orcasd").as_ref(), "daemon");
        assert_eq!(component_label("orcas-daemon").as_ref(), "daemon");
        assert_eq!(component_label("orcas").as_ref(), "supervisor");
        assert_eq!(component_label("orcas-supervisor").as_ref(), "supervisor");
        assert_eq!(component_label("app-server").as_ref(), "app-server");
        assert_eq!(component_label("codex-app-server").as_ref(), "app-server");
    }

    #[test]
    fn boolish_flags_parse_common_values() {
        assert!(parse_boolish(Some("1"), false));
        assert!(parse_boolish(Some("true"), false));
        assert!(parse_boolish(Some("ON"), false));
        assert!(!parse_boolish(Some("0"), true));
        assert!(!parse_boolish(Some("false"), true));
        assert!(!parse_boolish(Some("off"), true));
        assert!(parse_boolish(Some("unexpected"), true));
        assert!(!parse_boolish(Some("unexpected"), false));
        assert!(parse_boolish(None, true));
        assert!(!parse_boolish(None, false));
    }

    #[test]
    fn component_aggregate_format_places_component_after_level() {
        let buffer = BufferMakeWriter::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_thread_names(false)
                .with_thread_ids(false)
                .with_writer(buffer.clone())
                .event_format(ComponentAggregateFormat::new(Cow::Borrowed("tui")))
                .with_ansi(false),
        );

        tracing::subscriber::with_default(subscriber, || {
            info!("hello");
        });

        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert!(
            output.contains(" INFO tui hello\n"),
            "unexpected formatted output: {output}"
        );
    }
}
