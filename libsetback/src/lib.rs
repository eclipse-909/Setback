#![feature(
	const_format_args,
	const_trait_impl,
	slice_partition_dedup,
	trait_alias,
)]

mod arena;
mod client;
mod constants;
mod error;
mod items;
mod lobby;
mod models;
mod player;
mod runtime;
mod ssp;
mod telemetry;
mod ws;

#[cfg(feature = "server")]
mod server;

pub use {
	arena::*,
	client::*,
	constants::*,
	error::*,
	items::*,
	lobby::*,
	models::*,
	player::*,
	runtime::*,
	ssp::*,
	ws::{WebSocketeer, SspTx, FallbackRx, ws_local_connect},
	telemetry::*,
};

#[cfg(feature = "server")]
pub use server::*;