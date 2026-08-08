use dioxus::prelude::*;

const KEYBIND_ROW_CSS: Asset = asset!("/assets/styling/keybind-row.css");
const BUTTON_CSS: Asset = asset!("/assets/styling/button.css");

#[derive(Props, PartialEq, Clone)]
pub struct KeybindRowProps {
	action: String,
	default_key: String,
	set_key: String,
}

#[component]
pub fn KeybindRow(props: KeybindRowProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: KEYBIND_ROW_CSS }
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		div { class: "keybind-row", "data-default": "{props.default_key}",
			span { class: "keybind-row__action", "{props.action}" }
			div { class: "keybind-row__controls",
				button { class: "keycap", r#type: "button", "{props.set_key}" }
				button {
					aria_label: "Reset to default",
					class: "keybind-row__reset",
					r#type: "button",
					svg {
						fill: "none",
						stroke: "currentColor",
						stroke_width: "2",
						view_box: "0 0 24 24",
						path { d: "M3 12a9 9 0 1 0 3-6.7M3 4v5h5" }
					}
				}
			}
		}
	}
}