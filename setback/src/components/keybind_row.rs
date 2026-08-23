use {
	crate::components::*,
	dioxus::prelude::*,
	std::{
		fmt::{Display, Formatter},
		borrow::Cow,
	}
};

#[derive(Props, PartialEq, Clone)]
pub struct KeybindRowProps {
	action: KeyAction,
	default_key: Key,
	sig: Signal<Key>,
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
pub fn KeybindRow(mut props: KeybindRowProps) -> Element {
	let mut cap_txt = use_signal(|| Cow::Owned(props.default_key.to_string()));
	let mut cap_class = use_signal(|| "keycap");
	let mut sig_onkeydown = use_signal(|| None);
	rsx! {
		document::Link { rel: "stylesheet", href: KEYBIND_ROW_CSS }
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		div { class: "keybind-row",
			span { class: "keybind-row__action", "{props.action}" }
			div { class: "keybind-row__controls",
				button {
					class: "{cap_class}",
					r#type: "button",
					onclick: move |_| {
						cap_txt.set(Cow::Borrowed("..."));
						cap_class.set("keycap is-listening");
						sig_onkeydown.set(Some(move |e: KeyboardEvent| {
							e.prevent_default();
							let key = e.key();
							cap_txt.set(Cow::Owned(key.to_string()));
							*props.sig.write() = key;
							cap_class.set("keycap");
						}));
					},
					onkeydown: move |evt| if let Some(mut callback) = sig_onkeydown.take() {callback(evt)},

					"{cap_txt}"
				}
				button {
					aria_label: "Reset to default",
					class: "keybind-row__reset",
					r#type: "button",
					onclick: move |_| {
						cap_txt.set(Cow::Owned(props.default_key.to_string()));
						*props.sig.write() = props.default_key.clone();
						cap_class.set("keycap");
					},

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