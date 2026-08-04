#![feature(
	allocator_api,
	const_slice_make_iter,
	maybe_uninit_uninit_array_transpose,
	const_format_args,
	unsafe_cell_access,
	sync_unsafe_cell,
)]

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
mod client;

pub use {
	client::*,
	constants::*,
	error::*,
	items::*,
	models::*,
	player::*,
	runtime::*,
	setup::*,
	small_vec::*,
};