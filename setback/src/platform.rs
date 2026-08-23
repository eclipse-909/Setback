#[cfg(target_family = "wasm")]
pub use wasm::*;

#[cfg(not(target_family = "wasm"))]
pub use native::*;

// The public interface between these two platform-specific implementations must be identical

#[cfg(target_family = "wasm")]
mod wasm {
	pub async fn sleep(delay_ms: i32) {
		let delay_ms = if delay_ms < 0 {0} else {delay_ms};
		gloo::timers::future::TimeoutFuture::new(delay_ms as u32).await;
	}
}

#[cfg(not(target_family = "wasm"))]
mod native {
	pub async fn sleep(delay_ms: i32) {
		let delay_ms = if delay_ms < 0 {0} else {delay_ms};
		tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms as u64)).await;
	}
}