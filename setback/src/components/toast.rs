use {
	crate::{components::*, platform::*},
	dioxus::prelude::*,
	std::sync::atomic::{AtomicUsize, Ordering},
};

static TOASTS: GlobalSignal<Vec<Toast>> = Signal::global(Vec::new);
static NEXT_TOAST_ID: AtomicUsize = AtomicUsize::new(0);

struct Toast {
	id: usize,
	msg: String,
	is_err: bool,
	class: String,
}

#[component]
pub fn ToastRegion() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: TOAST_CSS }

		div { class: "toast-region",
			for toast in TOASTS.read().iter() {
				div {
					class: "{toast.class}",
					key: "toast-{toast.id}",

					svg {
						class: "toast__icon",
						view_box: "0 0 24 24",
						fill: "none",
						stroke: "currentColor",
						stroke_width: "2",

						if toast.is_err {
							circle { cx: "12", cy: "12", r: "9" }
							path { d: "M12 8v5M12 16h.01" }
						} else {
							path { d: "M20 6L9 17l-5-5" }
						}
					}
					span { "{toast.msg}" }
				}
			}
		}
	}
}

pub fn toast_success(message: impl Into<String>) { show_toast(message.into(), false); }

pub fn toast_err(message: impl Into<String>) { show_toast(message.into(), true); }

fn show_toast(message: String, is_err: bool) {
	let id = NEXT_TOAST_ID.fetch_add(1, Ordering::Relaxed);
	let err_class = if is_err { "toast--error" } else { "toast--success" };
	TOASTS.write().push(Toast { id, msg: message, is_err, class: format!("toast {err_class}") });

	spawn(async move {
		sleep(3400).await;
		// need to lookup the toast in the vec between .awaits because it's position can be changed from another running async task
		if let Some(toast) = TOASTS.write().iter_mut().rev().find(|t| t.id == id) {
			toast.class.push_str(" is-leaving");
		}
		sleep(250).await;
		TOASTS.write().retain(|t| t.id != id);
	});
}