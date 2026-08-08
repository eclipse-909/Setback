use dioxus::prelude::*;

const TOAST_CSS: Asset = asset!("/assets/styling/toast.css");

#[derive(Props, PartialEq, Clone)]
pub struct ToastProps {
	msg: String,
	toast_type: ToastType,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ToastType { Success, Error }

impl ToastType {
	const fn as_class(self) -> &'static str {
		match self {
			ToastType::Success => "toast--success",
			ToastType::Error => "toast--error",
		}
	}
}

#[component]
pub fn Toast(props: ToastProps) -> Element {
	rsx! {
        document::Link { rel: "stylesheet", href: TOAST_CSS }
        
		div { class: "toast {props.toast_type.as_class()}",
			svg {
				class: "toast__icon",
				fill: "none",
				stroke: "currentColor",
				stroke_width: "2",
				view_box: "0 0 24 24",
				circle { cx: "12", cy: "12", r: "9" }
				path { d: "M12 8v5M12 16h.01" }
			}
			span { "{props.msg}" }
		}
	}
}