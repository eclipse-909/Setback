use {
	crate::{Route, components::*},
	dioxus::prelude::*,
	libsetback::*,
};

#[component]
pub fn Join() -> Element {
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
				div { class: "panel panel--md",
					div { class: "panel__inner",
						div { class: "panel__header",
							p { class: "eyebrow", "Find a Seat" }
							h2 { class: "panel__title", "How would you like to join?" }
						}
						div { class: "panel__body",
							div { class: "option-grid",
								a {
									class: "option-card",
									href: "browse-games.html",
									span { class: "option-card__icon",
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
									span { class: "option-card__title", "Public" }
									span { class: "option-card__desc", "Browse open tables" }
								}
								button {
									class: "option-card",
									id: "privateOption",
									r#type: "button",
									span { class: "option-card__icon",
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
									span { class: "option-card__title", "Private" }
									span { class: "option-card__desc", "Enter a code & password" }
								}
							}
							div { hidden: "false", id: "privateJoinFields",
								div { class: "divider-suit",
									span { "♣" }
								}
								div { class: "stack gap-4",
									div { class: "field",
										label {
											class: "field__label",
											r#for: "joinCode",
											"Table Code"
										}
										input {
											class: "text-input text-input--code",
											id: "joinCode",
											maxlength: "6",
											placeholder: "AB12CD",
										}
									}
									div { class: "field",
										label {
											class: "field__label",
											r#for: "joinPassword",
											"Password"
										}
										input {
											class: "text-input",
											id: "joinPassword",
											placeholder: "Table password",
											r#type: "password",
										}
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
						div { class: "panel__footer",
							a {
								class: "btn btn--ghost btn--md",
								href: "multiplayer.html",
								"Cancel"
							}
							button {
								class: "btn btn--primary btn--md",
								hidden: "false",
								id: "joinPrivateBtn",
								"Join Table"
							}
						}
					}
				}
			}
		}
		ToastRegion {}
	}
}
