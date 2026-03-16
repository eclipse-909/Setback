use tokio::{
	runtime::{Builder, Runtime},
	task::JoinError,
};

struct Deferred<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Deferred<F> {
	fn drop(&mut self) {
		(
			// SAFETY:
			// function pointer should be initialized, otherwise it hopefully segfaults
			unsafe {self.0.take().unwrap_unchecked()}
		)()
	}
}

pub struct Global {
	pub rt: Runtime,
}

impl gpui::Global for Global {}

impl Default for Global {
	fn default() -> Self {
		Self {
			rt: Builder::new_multi_thread()
				.enable_all()
				.build()
				.expect("Failed to create tokio runtime"),
		}
	}
}

impl Global {
	pub fn spawn_tokio<R: Send + 'static>(
		&self,
		cx: &impl gpui::AppContext,
		future: impl Future<Output=R> + Send + 'static
	) -> gpui::Task<Result<R, JoinError>> {
		let join_handle = self.rt.spawn(future);
		let abort_handle = join_handle.abort_handle();
		let cancel = Deferred(Some(move || abort_handle.abort()));
		cx.background_spawn(async move {
			let result = join_handle.await?;
			drop(cancel);
			Ok(result)
		})
	}
}