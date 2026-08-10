use {
	crate::components::*,
	dioxus::prelude::*,
	std::fmt::{Display, Formatter},
};

#[derive(Props, PartialEq, Clone)]
pub struct KeybindRowProps {
	action: KeyAction,
	default_key: String,
	curr_key: String,
}

#[derive(PartialEq, Clone, Copy)]
pub enum KeyAction {
	Pass,
	Bid2,
	Bid3,
	Bid4,
	OverrideBid,
	PlayCard1,
	PlayCard2,
	PlayCard3,
	PlayCard4,
	PlayCard5,
	PlayCard6,
	ToggleSettings,
}

impl Display for KeyAction {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				KeyAction::Pass => "Pass",
				KeyAction::Bid2 => "Bid 2",
				KeyAction::Bid3 => "Bid 3",
				KeyAction::Bid4 => "Bid 4",
				KeyAction::OverrideBid => "Override Bid",
				KeyAction::PlayCard1 => "Play Card 1",
				KeyAction::PlayCard2 => "Play Card 2",
				KeyAction::PlayCard3 => "Play Card 3",
				KeyAction::PlayCard4 => "Play Card 4",
				KeyAction::PlayCard5 => "Play Card 5",
				KeyAction::PlayCard6 => "Play Card 6",
				KeyAction::ToggleSettings => "Toggle Settings",
			}
		)
	}
}

#[component]
pub fn KeybindRow(props: KeybindRowProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: KEYBIND_ROW_CSS }
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		div { class: "keybind-row", "data-default": "{props.default_key}",
			span { class: "keybind-row__action", "{props.action}" }
			div { class: "keybind-row__controls",
				button { class: "keycap", r#type: "button", "{props.curr_key}" }
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
