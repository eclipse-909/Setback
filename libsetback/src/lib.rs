#![feature(
	const_format_args,
	allocator_api,
	unsafe_cell_access,
	slice_partition_dedup,
	trait_alias,
	async_drop,
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
mod ws;

#[cfg(not(feature = "client"))]
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
};

use std::{
	future::AsyncDrop,
	pin::Pin,
};

#[cfg(not(feature = "client"))]
pub use server::*;

pub struct DeferredAsync<F: AsyncFnMut() + Unpin>(F);

impl<F: AsyncFnMut() + Unpin> AsyncDrop for DeferredAsync<F> {
	async fn drop(self: Pin<&mut Self>) {
		(self.get_mut().0)().await
	}
}

impl<F: AsyncFnMut() + Unpin> Drop for DeferredAsync<F> {
	fn drop(&mut self) {}
}

/// Only use this once per scope.
#[macro_export]
macro_rules! defer_async {
    ($e: expr) => {
        let _d = DeferredAsync(async || { $e });
    };
}