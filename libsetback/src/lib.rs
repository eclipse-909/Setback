#![feature(
	allocator_api,
	slice_ptr_get,
	slice_swap_unchecked,
	const_slice_make_iter,
	iter_map_windows,
	maybe_uninit_uninit_array_transpose,
	const_format_args,
)]

mod arena;
mod codec;
mod connection;
mod constants;
mod error;
mod items;
mod models;
mod player;
mod runtime;
mod sdp;
mod setup;
mod small_vec;
mod ssp;

pub use {
	arena::*, connection::*, constants::*, setup::*, items::*, models::*, player::*, runtime::*,
	ssp::*, small_vec::*, error::*
};
