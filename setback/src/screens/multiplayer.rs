use {crate::{components::*, *}, dioxus::prelude::*, setback::*};

#[component]
pub fn Multiplayer() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: PANEL_CSS }

		BackBtn { to: Route::MainMenu }
		main { class: "app-shell",
			div { class: "screen",
				div { class: "top-bar",
					h1 { class: "top-bar__title", "Multiplayer" }
				}
				Panel {
					div { class: "panel__inner",
						PanelHeader {
							eyebrow: "Take a Seat",
							title: "Host or Join",
							subtitle: "Start your own table, or find one already dealt.",
						}
						div { class: "panel__body",
							OptionCardGrid {
								OptionCard {
									to: Route::HostGame,
									title: "Host",
									description: "Create a public or private table",
									svg {
										fill: "none",
										stroke: "currentColor",
										stroke_width: "1.6",
										view_box: "0 0 24 24",
										path { d: "M4 21V8l8-5 8 5v13" }
										path { d: "M9 21v-6h6v6" }
									}
								}
								OptionCard {
									to: Route::Join,
									title: "Join",
									description: "Browse public tables or enter a code",
									svg {
										fill: "none",
										stroke: "currentColor",
										stroke_width: "1.6",
										view_box: "0 0 24 24",
										path { d: "M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" }
										path { d: "M10 17l5-5-5-5" }
										path { d: "M15 12H3" }
									}
								}
							}
						}
					}
				}
			}
		}
	}
}