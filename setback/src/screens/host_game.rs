use {
	crate::{Route, components::*},
	dioxus::prelude::*,
};

const PUBLIC: usize = 0;
const PRIVATE: usize = 1;

#[component]
pub fn HostGame() -> Element {
	let mut selected_idx = use_signal(|| 0);
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
								selected: selected_idx() == PUBLIC,
								title: "Public",
								description: "Anyone can find and join your table",
								onclick: move |_| selected_idx.set(PUBLIC),

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
								selected: selected_idx() == PRIVATE,
								title: "Private",
								description: "Only players with the code & password can join",
								onclick: move |_| selected_idx.set(PRIVATE),

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
						if selected_idx() == PRIVATE {
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
}
