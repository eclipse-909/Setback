use {crate::components::*, dioxus::prelude::*, libsetback::*};

#[derive(Props, PartialEq, Clone)]
pub struct SettingsProps {
	#[props(default)]
	done_onclick: Option<EventHandler<MouseEvent>>,
	in_game: bool,
}

#[component]
pub fn SettingsPanel(props: SettingsProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: PANEL_CSS }
		document::Link { rel: "stylesheet", href: KEYBIND_ROW_CSS }
		document::Link { rel: "stylesheet", href: TOAST_CSS }
		document::Link { rel: "stylesheet", href: SETTINGS_CSS }

		div { class: "top-bar", style: "--topbar-max: 42rem;",
			h1 { class: "top-bar__title", "Settings" }
		}
		div { class: "panel panel--lg", style: "max-width: 42rem;",
			div { class: "panel__inner", {
				tabs! [
					{
						"Video",
						div { class: "settings-row",
							span { class: "settings-row__label",
								"Display Mode"
								small { "Play in fullscreen or a windowed frame" }
							}
							Toggle { label: "Fullscreen" }
						}
						div {
							class: "settings-row",
							style: "flex-direction: column; align-items: stretch;",
							Slider {
								max: 150,
								min: 75,
								value: 100,
								step: 5,
								percentage: true,

								"UI Scale"
								br {}
								small { style: "font-weight:400;color:var(--cream-700);",
									"Resize menus, text, and cards"
								}
							}
						}
					},
					{
						"Audio",
						div { class: "stack gap-5",
							Slider {
								max: 100,
								min: 0,
								value: 100,
								step: 1,
								percentage: true,
								"Master Volume"
							}
							Slider {
								max: 100,
								min: 0,
								value: 100,
								step: 1,
								percentage: true,
								"Music Volume"
							}
							Slider {
								max: 100,
								min: 0,
								value: 100,
								step: 1,
								percentage: true,
								"Effects Volume"
							}
						}
					},
					{
						"Keybinds",
						div { class: "keybind-list",
							p { class: "keybind-group-title", "Bidding & Turn Actions" }
							KeybindRow {
								action: KeyAction::Pass,
								default_key: "P",
								curr_key: "P"
							}
							KeybindRow {
								action: KeyAction::Bid2,
								default_key: "2",
								curr_key: "2"
							}
							KeybindRow {
								action: KeyAction::Bid3,
								default_key: "3",
								curr_key: "3"
							}
							KeybindRow {
								action: KeyAction::Bid4,
								default_key: "4",
								curr_key: "4"
							}
							KeybindRow {
								action: KeyAction::OverrideBid,
								default_key: "4",
								curr_key: "4"
							}
							p { class: "keybind-group-title", "Playing Cards" }
							KeybindRow {
								action: KeyAction::PlayCard1,
								default_key: "1",
								curr_key: "1"
							}
							KeybindRow {
								action: KeyAction::PlayCard2,
								default_key: "2",
								curr_key: "2"
							}
							KeybindRow {
								action: KeyAction::PlayCard3,
								default_key: "3",
								curr_key: "3"
							}
							KeybindRow {
								action: KeyAction::PlayCard4,
								default_key: "4",
								curr_key: "4"
							}
							KeybindRow {
								action: KeyAction::PlayCard5,
								default_key: "5",
								curr_key: "5"
							}
							KeybindRow {
								action: KeyAction::PlayCard6,
								default_key: "6",
								curr_key: "6"
							}
							p { class: "keybind-group-title", "Interface" }
							KeybindRow {
								action: KeyAction::ToggleSettings,
								default_key: "esc",
								curr_key: "esc"
							}
						}
					},
					{
						if props.in_game { "Game" } else { "Account" },
						if props.in_game {
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Session"
									small { "If you deliberately leave the game you cannot return" }
								}
								Btn {
									variant: BtnVariant::Danger,
									size: BtnSize::Sm,
									//TODO onclick leave game
									"Leave Game"
								}
							}
						} else {
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Signed in as"
									small { "ridge_holler" }
								}
							}
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Website"
									small { "Visit our website in a new tab" }
								}
								Btn {
									variant: BtnVariant::Link,
									size: BtnSize::Sm,
									//TODO onclick open site
									"setback.eclipse-909.com"
								}
							}
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Remember Login"
									small { "Stay signed in on this device" }
								}
								Toggle {}
							}
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Session"
									small { "Sign out of your account on this device" }
								}
								Btn {
									variant: BtnVariant::Danger,
									size: BtnSize::Sm,
									//TODO onclick logout
									"Log Out"
								}
							}
						}
					},
				] }
			}
			div { class: "panel__footer",
				Btn {
					variant: BtnVariant::Primary,
					onclick: props.done_onclick,
					"Done"
				}
			}
		}
	}
}
