//! Shared tracing setup so every binary logs in the same format.
//!
//! Cargo.toml for the shared crate:
//!
//! ```toml
//! [dependencies]
//! tracing = "0.1"
//! tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
//! tower-http = { version = "0.6", features = ["trace"], optional = true }
//!
//! [features]
//! http = ["dep:tower-http"]
//! ```
//!
//! Env vars (set in `.env.local`):
//! - `RUST_LOG`   : standard filter syntax, e.g. `debug,sqlx=warn`. Defaults to `info`.
//! - `LOG_FORMAT` : `json` (default) or `pretty` for human-readable local output.

use {
	tracing_subscriber::{fmt, prelude::*, EnvFilter},
	std::{env, backtrace::Backtrace},
	tracing::{info, error},
};

/// Used when `RUST_LOG` is unset.
const DEFAULT_FILTER: &str = "info";

/// Initialize the global tracing subscriber. Call once, first thing in `main`.
///
/// Output goes to stdout so `docker compose logs` picks it up. Events from
/// crates that use the `log` facade (sqlx, hyper, etc.) are bridged
/// automatically by `tracing-subscriber`'s default `tracing-log` feature.
pub fn _init_tracing(service: &'static str) {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
	
	let registry = tracing_subscriber::registry()
		.with(filter);
	
	match env::var("LOG_FORMAT").as_deref() {
		Ok("pretty") => registry.with(fmt::layer().with_target(true)).init(),
		_ => registry
			.with(
				fmt::layer()
					.json()
					.flatten_event(true)
					.with_current_span(true)
					.with_span_list(false),
			)
			.init(),
	}
	
	std::panic::set_hook(Box::new(|info| {
		error!(panic = %info, "thread panicked");
		error!("Backtrace:\n{}", Backtrace::capture());
	}));
	
	info!(service, "logging initialised");
}

#[macro_export]
macro_rules! init_tracing {
    () => { _init_tracing(env!("CARGO_CRATE_NAME")) };
}

/// Same as `init_tracing`, but returns an error instead of panicking if a
/// subscriber is already set. Handy in tests, where many tests may call init.
pub fn try_init_tracing() -> Result<(), tracing_subscriber::util::TryInitError> {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
	
	tracing_subscriber::registry()
		.with(filter)
		.with(fmt::layer().with_test_writer())
		.try_init()
}

/// A `TraceLayer` with consistent settings for every axum service.
///
/// tower-http logs request spans and responses at DEBUG by default, which the
/// `info` default filter would hide. This raises both to INFO.
#[cfg(feature = "http")]
pub fn http_trace_layer() -> tower_http::trace::TraceLayer<
	tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
	tower_http::trace::DefaultMakeSpan,
	tower_http::trace::DefaultOnRequest,
	tower_http::trace::DefaultOnResponse,
> {
	use tower_http::{
		trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
		LatencyUnit,
	};
	use tracing::Level;
	
	TraceLayer::new_for_http()
		.make_span_with(DefaultMakeSpan::new().level(Level::INFO))
		.on_response(
			DefaultOnResponse::new()
				.level(Level::INFO)
				.latency_unit(LatencyUnit::Millis),
		)
}