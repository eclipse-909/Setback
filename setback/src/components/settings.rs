use {
	crate::{UI_SCALE, components::*},
	dioxus::prelude::*,
	setback::*,
};

//TODO these values need to be loaded from the user's saved settings
// rather than given a hard-coded initial value.

static FS_TOGGLE: GlobalSignal<bool> = Signal::global(|| false);
// ui scale is a global signal in main.rs

static MASTER_VOL: GlobalSignal<u8> = Signal::global(|| 100);
static MUSIC_VOL: GlobalSignal<u8> = Signal::global(|| 100);
static FX_VOL: GlobalSignal<u8> = Signal::global(|| 100);

// character keys are HEAP ALLOCATED STRINGS usually with only ONE character
// so we have to do all this shit to setup keybinds
const KEY_PASS_DEFAULT: fn() -> Key = || Key::Character(String::from("p"));
const KEY_BID2_DEFAULT: fn() -> Key = || Key::Character(String::from("2"));
const KEY_BID3_DEFAULT: fn() -> Key = || Key::Character(String::from("3"));
const KEY_BID4_DEFAULT: fn() -> Key = || Key::Character(String::from("4"));
const KEY_BID_OVERRIDE_DEFAULT: fn() -> Key = || Key::Character(String::from("4"));
const KEY_PLAY_CARD1_DEFAULT: fn() -> Key = || Key::Character(String::from("1"));
const KEY_PLAY_CARD2_DEFAULT: fn() -> Key = || Key::Character(String::from("2"));
const KEY_PLAY_CARD3_DEFAULT: fn() -> Key = || Key::Character(String::from("3"));
const KEY_PLAY_CARD4_DEFAULT: fn() -> Key = || Key::Character(String::from("4"));
const KEY_PLAY_CARD5_DEFAULT: fn() -> Key = || Key::Character(String::from("5"));
const KEY_PLAY_CARD6_DEFAULT: fn() -> Key = || Key::Character(String::from("6"));
const KEY_TOGGLE_SETTINGS_DEFAULT: Key = Key::Escape;

static KEY_PASS: GlobalSignal<Key> = Signal::global(KEY_PASS_DEFAULT);
static KEY_BID2: GlobalSignal<Key> = Signal::global(KEY_BID2_DEFAULT);
static KEY_BID3: GlobalSignal<Key> = Signal::global(KEY_BID3_DEFAULT);
static KEY_BID4: GlobalSignal<Key> = Signal::global(KEY_BID4_DEFAULT);
static KEY_BID_OVERRIDE: GlobalSignal<Key> = Signal::global(KEY_BID_OVERRIDE_DEFAULT);
static KEY_PLAY_CARD1: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD1_DEFAULT);
static KEY_PLAY_CARD2: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD2_DEFAULT);
static KEY_PLAY_CARD3: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD3_DEFAULT);
static KEY_PLAY_CARD4: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD4_DEFAULT);
static KEY_PLAY_CARD5: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD5_DEFAULT);
static KEY_PLAY_CARD6: GlobalSignal<Key> = Signal::global(KEY_PLAY_CARD6_DEFAULT);
static KEY_TOGGLE_SETTINGS: GlobalSignal<Key> = Signal::global(|| KEY_TOGGLE_SETTINGS_DEFAULT);

static REMEMBER_LOGIN: GlobalSignal<bool> = Signal::global(|| true);

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
							Toggle {
								label: if FS_TOGGLE() {"Fullscreen"} else {"Windowed"},
								checked: FS_TOGGLE(),
								oninput: move |checked| *FS_TOGGLE.write() = checked,
							}
						}
						div {
							class: "settings-row",
							style: "flex-direction: column; align-items: stretch;",
							Slider {
								max: 150,
								min: 75,
								value: (UI_SCALE() * 100.) as u8,
								step: 5,
								percentage: true,
								oninput: move |value| *UI_SCALE.write() = value as f32 / 100.,

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
								value: MASTER_VOL(),
								step: 1,
								percentage: true,
								oninput: move |value| *MASTER_VOL.write() = value,

								"Master Volume"
							}
							Slider {
								max: 100,
								min: 0,
								value: MUSIC_VOL(),
								step: 1,
								percentage: true,
								oninput: move |value| *MUSIC_VOL.write() = value,

								"Music Volume"
							}
							Slider {
								max: 100,
								min: 0,
								value: FX_VOL(),
								step: 1,
								percentage: true,
								oninput: move |value| *FX_VOL.write() = value,

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
								default_key: KEY_PASS_DEFAULT(),
								sig: KEY_PASS.signal(),
							}
							KeybindRow {
								action: KeyAction::Bid2,
								default_key: KEY_BID2_DEFAULT(),
								sig: KEY_BID2.signal(),
							}
							KeybindRow {
								action: KeyAction::Bid3,
								default_key: KEY_BID3_DEFAULT(),
								sig: KEY_BID3.signal(),
							}
							KeybindRow {
								action: KeyAction::Bid4,
								default_key: KEY_BID4_DEFAULT(),
								sig: KEY_BID4.signal(),
							}
							KeybindRow {
								action: KeyAction::OverrideBid,
								default_key: KEY_BID_OVERRIDE_DEFAULT(),
								sig: KEY_BID_OVERRIDE.signal(),
							}
							p { class: "keybind-group-title", "Playing Cards" }
							KeybindRow {
								action: KeyAction::PlayCard1,
								default_key: KEY_PLAY_CARD1_DEFAULT(),
								sig: KEY_PLAY_CARD1.signal(),
							}
							KeybindRow {
								action: KeyAction::PlayCard2,
								default_key: KEY_PLAY_CARD2_DEFAULT(),
								sig: KEY_PLAY_CARD2.signal(),
							}
							KeybindRow {
								action: KeyAction::PlayCard3,
								default_key: KEY_PLAY_CARD3_DEFAULT(),
								sig: KEY_PLAY_CARD3.signal(),
							}
							KeybindRow {
								action: KeyAction::PlayCard4,
								default_key: KEY_PLAY_CARD4_DEFAULT(),
								sig: KEY_PLAY_CARD4.signal(),
							}
							KeybindRow {
								action: KeyAction::PlayCard5,
								default_key: KEY_PLAY_CARD5_DEFAULT(),
								sig: KEY_PLAY_CARD5.signal(),
							}
							KeybindRow {
								action: KeyAction::PlayCard6,
								default_key: KEY_PLAY_CARD6_DEFAULT(),
								sig: KEY_PLAY_CARD6.signal(),
							}
							p { class: "keybind-group-title", "Interface" }
							KeybindRow {
								action: KeyAction::ToggleSettings,
								default_key: KEY_TOGGLE_SETTINGS_DEFAULT,
								sig: KEY_TOGGLE_SETTINGS.signal(),
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
									onclick: move |_| {
										// TODO leave game
									},
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
									small {
										if cfg!(target_family = "wasm") {
											"Visit our website in a new tab"
										} else {
											"Visit our website in the browser"
										}
									}
								}
								AnchorBtn {
									size: BtnSize::Sm,
									href: "https://eclipse-909.com",
									target: "_blank",
									"https://eclipse-909.com"
								}
							}
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Remember Login"
									small { "Stay signed in on this device" }
								}
								Toggle {
									oninput: move |checked| *REMEMBER_LOGIN.write() = checked
								}
							}
							div { class: "settings-row",
								span { class: "settings-row__label",
									"Session"
									small { "Sign out of your account on this device" }
								}
								Btn {
									variant: BtnVariant::Danger,
									size: BtnSize::Sm,
									onclick: move |_| {
										//TODO logout
									},
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
