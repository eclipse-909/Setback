#![feature(
	const_format_args,
	allocator_api,
	unsafe_cell_access,
	slice_partition_dedup,
	trait_alias,
	const_option_ops,
	const_trait_impl,
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
	ws::{WebSocketeer, SspTx, FallbackRx},
	telemetry::*,
};

#[cfg(feature = "server")]
pub use server::*;