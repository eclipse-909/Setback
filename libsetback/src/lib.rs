#![feature(
	allocator_api,
	const_slice_make_iter,
	maybe_uninit_uninit_array_transpose,
	const_format_args,
	unsafe_cell_access,
	sync_unsafe_cell,
)]

mod arena;
mod connection;
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

#[cfg(not(feature = "client"))]
mod state;

#[cfg(feature = "gpui")]
mod global;

pub(crate) use {
	player::*,
	setup::*,
};

pub use {
	arena::*,
	connection::{setback_api, SetbackClient},
	constants::*,
	error::Error,
	items::*,
	models::*,
	player::GameEventEmitter,
	runtime::GameState,
	setup::GameSetup,
	small_vec::*,
};

#[cfg(not(feature = "client"))]
pub use {
	state::*,
	connection::ClientConnection,
	runtime::GameLoopHandle
};

#[cfg(feature = "gpui")]
pub use {
	global::*,
	player::GpuiEventEmitter
};