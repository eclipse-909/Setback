#![feature(
	const_slice_make_iter,
	maybe_uninit_uninit_array_transpose,
	const_format_args,
	sync_unsafe_cell,
	allocator_api,
	unsafe_cell_access,
)]

mod arena;
mod client;
mod constants;
mod error;
mod items;
mod models;
mod player;
mod runtime;
pub mod sdp;
mod setup;
mod small_vec;
pub mod ssp;
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
	small_vec::*,
	ws::*,
};

#[cfg(not(feature = "client"))]
pub use {
	server_net::*,
	server_rt::*,
};