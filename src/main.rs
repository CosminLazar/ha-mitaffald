use ha_mitaffald::homeassistant::HASensor;
use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;
use opentelemetry::trace::Tracer;
use opentelemetry_otlp::WithExportConfig;
use std::collections::HashMap;
use tracing::{error, info, span, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{util::SubscriberInitExt, FmtSubscriber, Layer};

// #[tokio::main]

fn main() {
    config();
    
    info!("Starting");
    let tracer = opentelemetry::global::tracer("ex.com/basic");
    tracer.in_span("operation", |cx| {
        info!(target: "my-target", "hello from {}. My price is {}. I am also inside a Span!", "banana", 2.99);
    

    info!("heelo world!");
    info!(
        // another_another_dude = "value2",
        "Number of Lebowski references: {references} with {another_dude}",
        references = 1,
        another_dude = "value",
    );
    let settings = Settings::new().expect("Failed to read settings");
    let mut sensor_map: HashMap<String, HASensor> = HashMap::new();

    let report = sync_data(settings, &mut sensor_map);

    if let Err(x) = report {
        eprintln!(
            "Failure while reporting data (some entities may have been updated): {}",
            x
        );
    }
});
    //sleep 5 seconds
    println!("Sleeping 5 seconds");
    let five_seconds = std::time::Duration::from_secs(5);
    std::thread::sleep(five_seconds);
    opentelemetry::global::shutdown_tracer_provider();
    opentelemetry::global::shutdown_logger_provider();
    error!("This event will be logged in the root span.");
}

fn grafana_new(headers: HashMap<String, String>) -> opentelemetry_sdk::trace::Tracer {
    let grafana_exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_headers(headers.clone())
        .with_endpoint("https://otlp-gateway-prod-eu-west-2.grafana.net/otlp");

    // Then pass it into pipeline builder
    let resource = opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        "ha_mitaffald",
    )]);

    let tracing_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        // .with_exporter(otlp_exporter)
        .with_exporter(grafana_exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default().with_resource(resource.clone()),
        )
        .install_simple()
        .unwrap();

    return tracing_tracer;
}

fn init_logs(
    headers: HashMap<String, String>,
) -> Result<opentelemetry_sdk::logs::Logger, opentelemetry::logs::LogError> {
    let service_name = env!("CARGO_BIN_NAME");
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(opentelemetry_sdk::logs::Config::default().with_resource(
            opentelemetry_sdk::Resource::new(vec![
                opentelemetry::KeyValue::new("service.name", service_name),
                opentelemetry::KeyValue::new("service.customvalue", "custom-value"),
            ]),
        ))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint("https://otlp-gateway-prod-eu-west-2.grafana.net/otlp")
                .with_headers(headers)
                .with_protocol(opentelemetry_otlp::Protocol::HttpBinary),
        )
        // .with_exporter(opentelemetry_stdout::LogExporter::default())
        .install_simple()
}

fn config() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_owned(), "Basic Nzc0NzM2OmdsY19leUp2SWpvaU9UYzJNamd4SWl3aWJpSTZJbWhoTFcxcGRHRm1abUZzWkMxb1lTMXRhWFJoWm1aaGJHUWlMQ0pySWpvaU5UaHZNM0l4TnpOQ1pYVkZVVWhVVlRBM2JURnVabEV5SWl3aWJTSTZleUp5SWpvaWRYTWlmWDA9".to_owned());

    //trace/log shipping generates logs, remember to deactivate if lowering the log level
    init_logs(headers.clone()).unwrap();
    // configure the global logger to use our opentelemetry logger
    let logger_provider = opentelemetry::global::logger_provider();
    let log_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider)
            .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    //grafana_new(headers);
    let trace_layer = tracing_opentelemetry::layer()
        .with_tracer(grafana_new(headers))
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    // opentelemetry::global::set_text_map_propagator(
    //     opentelemetry::sdk::propagation::TraceContextPropagator::new(),
    // );

    // let x = tracing_subscriber::fmt();
    // let xx = x.json();

    tracing_subscriber::registry()
        .with(trace_layer)
        .with(log_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(tracing_subscriber::filter::LevelFilter::INFO),
        )
        .init();

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    //https://www.aspecto.io/blog/distributed-tracing-with-opentelemetry-rust/
}

// fn init_trace() -> opentelemetry::sdk::trace::TracerProvider {
//     let exporter = opentelemetry_stdout::LogExporter::default();
//     let processor =
//         opentelemetry::sdk::trace::BatchSpanProcessor::builder(exporter, runtime::Tokio).build();
//     opentelemetry::sdk::trace::TracerProvider::builder()
//         .with_span_processor(processor)
//         .build()
// }
