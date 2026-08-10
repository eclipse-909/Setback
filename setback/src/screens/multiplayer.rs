use {crate::components::*, dioxus::prelude::*, libsetback::*};

#[component]
pub fn Multiplayer() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: PANEL_CSS }

		a {
			aria_label: "Back to main menu",
			class: "btn btn--ghost btn--icon back-button",
			href: "../index.html",
			svg {
				fill: "none",
				stroke: "currentColor",
				stroke_width: "2",
				view_box: "0 0 24 24",
				path { d: "M15 18l-6-6 6-6" }
			}
		}
		main { class: "app-shell",
			div { class: "screen",
				div { class: "top-bar",
					h1 { class: "top-bar__title", "Multiplayer" }
				}
				div { class: "panel panel--md",
					div { class: "panel__inner",
						div { class: "panel__header",
							p { class: "eyebrow", "Take a Seat" }
							h2 { class: "panel__title", "Host or Join" }
							p { class: "panel__subtitle",
								"Start your own table, or find one already dealt."
							}
						}
						div { class: "panel__body",
							div { class: "option-grid",
								a {
									class: "option-card",
									href: "host-game.html",
									span { class: "option-card__icon",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "1.6",
											view_box: "0 0 24 24",
											path { d: "M4 21V8l8-5 8 5v13" }
											path { d: "M9 21v-6h6v6" }
										}
									}
									span { class: "option-card__title", "Host" }
									span { class: "option-card__desc",
										"Create a public or private table"
									}
								}
								a {
									class: "option-card",
									href: "join.html",
									span { class: "option-card__icon",
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
									span { class: "option-card__title", "Join" }
									span { class: "option-card__desc",
										"Browse public tables or enter a code"
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
