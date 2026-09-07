#![feature(
	const_format_args,
	allocator_api,
	unsafe_cell_access,
	slice_partition_dedup,
	trait_alias,
)]

mod arena;
mod client;
mod constants;
mod error;
mod items;
mod models;
mod player;
mod runtime;
mod setup;
mod ssp;
mod ws;

#[cfg(not(feature = "client"))]
mod server_net;
#[cfg(not(feature = "client"))]
mod server_rt;

pub use {
	arena::*,
	client::*,
	constants::*,
	error::*,
	items::*,
	models::*,
	player::*,
	runtime::*,
	setup::*,
	ssp::*,
	ws::{WebSocketeer, SspTx, FallbackRx},
};

#[cfg(not(feature = "client"))]
pub use {server_net::*, server_rt::*};
