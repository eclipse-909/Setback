use {
	crate::{Route, components::*},
	dioxus::prelude::*,
	libsetback::*,
};

#[component]
pub fn Join() -> Element {
	let mut private_selected = use_signal(|| false);
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: PANEL_CSS }
		document::Link { rel: "stylesheet", href: TEXT_INPUT_CSS }
		document::Link { rel: "stylesheet", href: TOAST_CSS }

		BackBtn { to: Route::Multiplayer }
		main { class: "app-shell",
			div { class: "screen",
				div { class: "top-bar",
					h1 { class: "top-bar__title", "Join a Table" }
				}
				Panel {
					PanelInner {
						eyebrow: "Find a Seat",
						title: "How would you like to join?",
						ghost_btn_txt: "Cancel",
						primary_btn_txt: "Join Table",

						OptionCardGrid {
							OptionCard {
								to: Route::BrowseGames,
								title: "Public",
								description: "Browse open tables",
								svg {
									fill: "none",
									stroke: "currentColor",
									stroke_width: "1.6",
									view_box: "0 0 24 24",
									circle { cx: "12", cy: "12", r: "9" }
									path { d: "M3 12h18M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18z" }
								}
							}
							OptionCard {
								selected: private_selected(),
								title: "Private",
								description: "Enter a code & password",
								onclick: move |_| private_selected.set(!private_selected()),
								svg {
									fill: "none",
									stroke: "currentColor",
									stroke_width: "1.6",
									view_box: "0 0 24 24",
									rect { height: "9", rx: "2", width: "16", x: "4", y: "11" }
									path { d: "M8 11V7a4 4 0 0 1 8 0v4" }
								}
							}
						}
						if private_selected() {
							div { id: "privateJoinFields",
								div { class: "divider-suit",
									span { "♣" }
								}
								div { class: "stack gap-4",
									TextInput {
										label: "Table Code",
										placeholder: "AB12CD",
										input_type: TextInputType::Code,
									}
									TextInput {
										label: "Password",
										placeholder: "Table password",
										input_type: TextInputType::Password,
									}
									p { class: "field__hint",
										"Demo credentials: code "
										strong { "DEMO1" }
										", password "
										strong { "setback" }
									}
								}
							}
						}
					}
				}
			}
		}
		ToastRegion {}
	}
}