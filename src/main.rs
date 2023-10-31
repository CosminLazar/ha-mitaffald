use ha_mitaffald::homeassistant::HASensor;
use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;
use opentelemetry::logs::{LogRecord, Logger};
use opentelemetry_otlp::WithExportConfig;
use std::collections::HashMap;
use tracing::{info, Level};
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt, FmtSubscriber, Layer,
};

// #[tokio::main]
fn main() {
    config();
    info!("Starting");

    info!("heelo world!");

    let settings = Settings::new().expect("Failed to read settings");
    let mut sensor_map: HashMap<String, HASensor> = HashMap::new();

    let report = sync_data(settings, &mut sensor_map);

    if let Err(x) = report {
        eprintln!(
            "Failure while reporting data (some entities may have been updated): {}",
            x
        );
    }

    opentelemetry::global::shutdown_tracer_provider();
    opentelemetry::global::shutdown_logger_provider();
}

fn grafana_new(headers: HashMap<String, String>) -> opentelemetry::sdk::trace::Tracer {
    let grafana_exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_headers(headers.clone())
        .with_endpoint("https://otlp-gateway-prod-eu-west-2.grafana.net/otlp/v1/traces");

    // Then pass it into pipeline builder
    let resource = opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        "ha_mitaffald",
    )]);

    let tracing_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        // .with_exporter(otlp_exporter)
        .with_exporter(grafana_exporter)
        .with_trace_config(
            opentelemetry::sdk::trace::Config::default().with_resource(resource.clone()),
        )
        .install_simple()
        .unwrap();

    return tracing_tracer;
}

fn init_logs(
    headers: HashMap<String, String>,
) -> Result<opentelemetry::sdk::logs::Logger, opentelemetry::logs::LogError> {
    let service_name = env!("CARGO_BIN_NAME");
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(opentelemetry::sdk::logs::Config::default().with_resource(
            opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                service_name,
            )]),
        ))
        // .with_exporter(exporter)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint("https://otlp-gateway-prod-eu-west-2.grafana.net/otlp/v1/logs")
                .with_headers(headers)
                .with_protocol(opentelemetry_otlp::Protocol::HttpBinary),
        )
        .install_simple()
}

fn config() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_owned(), "".to_owned());

    //trace/log shipping generates logs, remember to deactivate if lowering the log level
    init_logs(headers.clone()).unwrap();
    // configure the global logger to use our opentelemetry logger
    let logger_provider = opentelemetry::global::logger_provider();
    let log_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider)
            .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    let trace_layer = tracing_opentelemetry::layer()
        .with_tracer(grafana_new(headers))
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(log_layer)
        .with(trace_layer)
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}
