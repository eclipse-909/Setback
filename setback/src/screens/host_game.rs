use {
	crate::{Route, components::*},
	dioxus::prelude::*,
	libsetback::*,
};

#[component]
pub fn HostGame() -> Element {
	rsx! {
		BackBtn { to: Route::Multiplayer }
		main { class: "app-shell",
			div { class: "screen",
				div { class: "top-bar",
					h1 { class: "top-bar__title", "Host a Table" }
				}
				Panel {
					PanelInner {
						eyebrow: "Deal Yourself In",
						title: "Table Visibility",
						ghost_btn_txt: "Cancel",
						primary_btn_txt: "Start Table",

						OptionCardGrid {
							OptionCard {
								selected: true,
								title: "Public",
								description: "Anyone can find and join your table",

								svg {
									fill: "none",
									stroke: "currentColor",
									stroke_width: "1.6",
									view_box: "0 0 24 24",
									circle {
										cx: "12",
										cy: "12",
										r: "9",
									}
									path { d: "M3 12h18M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18z" }
								}
							}
							OptionCard {
								title: "Private",
								description: "Only players with the code & password can join",

								svg {
									fill: "none",
									stroke: "currentColor",
									stroke_width: "1.6",
									view_box: "0 0 24 24",
									rect {
										height: "9",
										rx: "2",
										width: "16",
										x: "4",
										y: "11",
									}
									path { d: "M8 11V7a4 4 0 0 1 8 0v4" }
								}
							}
						}
						//TODO this should only show if private is selected
						TextInput {
							label: "Set a Password",
							placeholder: "Choose a table password",
							input_type: TextInputType::Password,
							hint_msg: "Players will need this, plus the table code, to join.",
						}
					}
				}
			}
		}
	}
}
