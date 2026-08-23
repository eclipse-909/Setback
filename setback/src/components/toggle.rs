use {crate::components::*, dioxus::prelude::*};

#[derive(Props, PartialEq, Clone)]
pub struct ToggleProps {
	#[props(default)]
	checked: bool,
	#[props(default)]
	label: String,
	#[props(default)]
	description: String,
	#[props(default)]
	oninput: Option<EventHandler<bool>>
}

#[component]
pub fn Toggle(props: ToggleProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: TOGGLE_CSS }

		label { class: "toggle",
			input {
				checked: "{props.checked}",
				r#type: "checkbox",
				oninput: move |evt| if let Some(callback) = props.oninput { callback.call(evt.checked()) }
			}
			span { class: "toggle__track",
				span { class: "toggle__thumb" }
			}
			if !props.label.is_empty() {
				span { class: "toggle__label",
					"{props.label}"
					if !props.description.is_empty() {
						small { "{props.description}" }
					}
				}
			}
		}
	}
}
