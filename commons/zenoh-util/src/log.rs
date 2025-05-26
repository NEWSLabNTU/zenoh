//
// Copyright (c) 2024 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use std::{fmt, thread, thread::ThreadId};

use tracing::{field::Field, span, Event, Metadata, Subscriber};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};

/// A utility function to enable the tracing formatting subscriber.
///
/// The [`tracing_subscriber`]` is initialized from the `RUST_LOG` environment variable.
/// If `RUST_LOG` is not set, then logging is not enabled.
///
/// # Safety
///
/// Calling this function initializes a `lazy_static` in the [`tracing`] crate.
/// Such static is not deallocated prior to process exiting, thus tools such as `valgrind`
/// will report a memory leak.
/// Refer to this issue: <https://github.com/tokio-rs/tracing/issues/2069>
pub fn try_init_log_from_env() {
    if let Ok(env_filter) = EnvFilter::try_from_default_env() {
        init_env_filter(env_filter);
    }
}

/// A utility function to enable the tracing formatting subscriber.
///
/// The [`tracing_subscriber`] is initialized from the `RUST_LOG` environment variable.
/// If `RUST_LOG` is not set, then fallback directives are used.
///
/// # Safety
/// Calling this function initializes a `lazy_static` in the [`tracing`] crate.
/// Such static is not deallocated prior to process existing, thus tools such as `valgrind`
/// will report a memory leak.
/// Refer to this issue: <https://github.com/tokio-rs/tracing/issues/2069>
pub fn init_log_from_env_or<S>(fallback: S)
where
    S: AsRef<str>,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(fallback));
    init_env_filter(env_filter);
}

fn init_env_filter(env_filter: EnvFilter) {
    let fmt_layer = init_fmt_layer();
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    #[cfg(feature = "opentelemetry")]
    {
        match init_otlp_layer() {
            Ok(Some(otlp_layer)) => {
                registry.with(otlp_layer).init();
                eprintln!("Zenoh logging initialized (JSON + OpenTelemetry mode)");
                return;
            }
            Ok(None) => {
                registry.init();
                eprintln!("Zenoh logging initialized (JSON mode - no OTLP endpoint configured)");
                return;
            }
            Err(err) => {
                eprintln!("Failed to initialize OpenTelemetry layer: {}", err);
                registry.init();
                eprintln!("Zenoh logging initialized (JSON mode - OpenTelemetry fallback)");
                return;
            }
        };
    }

    #[cfg(not(feature = "opentelemetry"))]
    {
        registry.init();
        eprintln!("Zenoh logging initialized (JSON mode)");
    }
}

fn init_fmt_layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    tracing_subscriber::fmt::layer()
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_level(true)
        .with_target(true)
        .json()
}

#[cfg(feature = "opentelemetry")]
fn init_otlp_layer<S>() -> Result<
    Option<tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::SdkTracer>>,
    Box<dyn std::error::Error>,
>
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::SpanExporter;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{trace as sdktrace, Resource};

    let endpoint = match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) => endpoint,
        Err(_) => {
            // OTEL_EXPORTER_OTLP_ENDPOINT not set - this is expected for local-only mode
            return Ok(None);
        }
    };

    // Create OTLP exporter
    let otlp_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    // Get peer ID and experiment ID from environment
    let peer_id = std::env::var("PEER_ID").unwrap_or_else(|_| "unknown".to_string());
    let experiment_id = std::env::var("EXPERIMENT_ID").unwrap_or_else(|_| "unknown".to_string());

    // Create resource with service metadata
    let resource = Resource::builder()
        .with_service_name("zenoh-peer")
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .with_attribute(KeyValue::new("peer.id", peer_id))
        .with_attribute(KeyValue::new("experiment.id", experiment_id))
        .build();

    // Create tracer provider
    let provider = sdktrace::SdkTracerProvider::builder()
        .with_simple_exporter(otlp_exporter)
        .with_resource(resource)
        .build();

    // Set as global provider (optional but useful for propagation)
    opentelemetry::global::set_tracer_provider(provider.clone());

    // Get tracer
    let tracer = provider.tracer("zenoh");

    // Create layer
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Ok(Some(layer))
}

pub struct LogRecord {
    pub target: String,
    pub level: tracing::Level,
    pub file: Option<&'static str>,
    pub line: Option<u32>,
    pub thread_id: ThreadId,
    pub thread_name: Option<String>,
    pub message: Option<String>,
    pub attributes: Vec<(&'static str, String)>,
}

#[derive(Clone)]
struct SpanFields(Vec<(&'static str, String)>);

struct Layer<Enabled, Callback> {
    enabled: Enabled,
    callback: Callback,
}

impl<S, E, C> tracing_subscriber::Layer<S> for Layer<E, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    E: Fn(&Metadata) -> bool + 'static,
    C: Fn(LogRecord) + 'static,
{
    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, S>) -> bool {
        (self.enabled)(metadata)
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        let mut extensions = span.extensions_mut();
        let mut fields = vec![];
        attrs.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            fields.push((field.name(), format!("{value:?}")))
        });
        extensions.insert(SpanFields(fields));
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        let mut extensions = span.extensions_mut();
        let fields = extensions.get_mut::<SpanFields>().unwrap();
        values.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            fields.0.push((field.name(), format!("{value:?}")))
        });
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let thread = thread::current();
        let mut record = LogRecord {
            target: event.metadata().target().into(),
            level: *event.metadata().level(),
            file: event.metadata().file(),
            line: event.metadata().line(),
            thread_id: thread.id(),
            thread_name: thread.name().map(Into::into),
            message: None,
            attributes: vec![],
        };
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                let extensions = span.extensions();
                let fields = extensions.get::<SpanFields>().unwrap();
                record.attributes.extend(fields.0.iter().cloned());
            }
        }
        event.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            if field.name() == "message" {
                record.message = Some(format!("{value:?}"));
            } else {
                record.attributes.push((field.name(), format!("{value:?}")))
            }
        });
        (self.callback)(record);
    }
}

pub fn init_log_with_callback(
    enabled: impl Fn(&Metadata) -> bool + Send + Sync + 'static,
    callback: impl Fn(LogRecord) + Send + Sync + 'static,
) {
    let subscriber = tracing_subscriber::registry().with(Layer { enabled, callback });
    let _ = tracing::subscriber::set_global_default(subscriber);
}

#[cfg(feature = "test")]
// Used to verify memory leaks for valgrind CI.
// `EnvFilter` internally uses a static reference that is not cleaned up yielding to false positive in valgrind.
// This function enables logging without calling `EnvFilter` for env configuration.
pub fn init_log_test() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_level(true)
        .with_target(true);

    let subscriber = subscriber.finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}
