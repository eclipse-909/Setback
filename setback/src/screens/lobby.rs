use {crate::{components::*, *}, dioxus::prelude::*, setback::*};

const LOBBY_CSS: Asset = asset!("/assets/styling/lobby.css");

#[component]
pub fn Lobby() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: PANEL_CSS }
		document::Link { rel: "stylesheet", href: DROPDOWN_CSS }
		document::Link { rel: "stylesheet", href: TOGGLE_CSS }
		document::Link { rel: "stylesheet", href: INFO_TOOLTIP_CSS }
		document::Link { rel: "stylesheet", href: PLAYER_NODE_CSS }
		document::Link { rel: "stylesheet", href: LOBBY_CSS }

		BackBtn { to: Route::MainMenu }
		div { class: "lobby-page",
			div { class: "lobby-header",
				h1 { class: "lobby-header__title", "Table Setup" }
				div { class: "lobby-header__trailing",
					span { class: "code-badge", id: "codeBadge", "DEMO1" }
					label {
						class: "role-preview",
						title: "For reviewing this prototype only — not part of the real game.",
						"\n\t\t\t\tPreview\n\t\t\t\t"
						select { id: "rolePreview",
							option { value: "host", "Host (editable)" }
							option { value: "guest", "Guest (view only)" }
						}
					}
				}
			}
			div { class: "lobby-body",
				aside { class: "panel panel--sm lobby-options",
					div { class: "panel__inner",
						div {
							class: "panel__header",
							style: "margin-bottom: var(--space-3);",
							p { class: "eyebrow", "Table Rules" }
						}
						fieldset { id: "optionsFieldset",
							div { class: "option-row",
								span { class: "option-row__label",
									"\n              Teams Rule Set\n              "
									span { class: "info-icon", tabindex: "0",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "2.4",
											view_box: "0 0 24 24",
											circle {
												cx: "12",
												cy: "12",
												r: "10",
											}
											path { d: "M12 16v-4M12 8h.01" }
										}
										span { class: "info-icon__tip",
											"Standard pairs seats across the table into two fixed teams. Custom lets the host assign each seat's team by hand."
										}
									}
								}
								select { class: "dropdown", id: "teamsRuleSet",
									option {
										selected: "false",
										value: "standard",
										"Standard"
									}
									option { value: "custom", "Custom" }
								}
							}
							div { class: "option-row",
								span { class: "option-row__label",
									"\n              Win By 2\n              "
									span { class: "info-icon", tabindex: "0",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "2.4",
											view_box: "0 0 24 24",
											circle {
												cx: "12",
												cy: "12",
												r: "10",
											}
											path { d: "M12 16v-4M12 8h.01" }
										}
										span { class: "info-icon__tip",
											"The winning team must finish at least 2 points ahead. If they don't, play continues past the target score."
										}
									}
								}
								label { class: "toggle",
									input {
										checked: "false",
										id: "winByTwo",
										r#type: "checkbox",
									}
									span { class: "toggle__track",
										span { class: "toggle__thumb" }
									}
								}
							}
							div { class: "option-row",
								span { class: "option-row__label",
									"\n              Negative Scores\n              "
									span { class: "info-icon", tabindex: "0",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "2.4",
											view_box: "0 0 24 24",
											circle {
												cx: "12",
												cy: "12",
												r: "10",
											}
											path { d: "M12 16v-4M12 8h.01" }
										}
										span { class: "info-icon__tip",
											"Teams can drop below zero if they're set back by more than they've earned."
										}
									}
								}
								label { class: "toggle",
									input {
										checked: "false",
										id: "negativeScores",
										r#type: "checkbox",
									}
									span { class: "toggle__track",
										span { class: "toggle__thumb" }
									}
								}
							}
							div { class: "option-row",
								span { class: "option-row__label",
									"\n              Dealer Override\n              "
									span { class: "info-icon", tabindex: "0",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "2.4",
											view_box: "0 0 24 24",
											circle {
												cx: "12",
												cy: "12",
												r: "10",
											}
											path { d: "M12 16v-4M12 8h.01" }
										}
										span { class: "info-icon__tip",
											"If a player bids 4, the dealer may override the bid by bidding 4."
										}
									}
								}
								label { class: "toggle",
									input {
										id: "dealerOverride",
										r#type: "checkbox",
									}
									span { class: "toggle__track",
										span { class: "toggle__thumb" }
									}
								}
							}
							div { class: "option-row",
								span { class: "option-row__label",
									"\n              Play To\n              "
									span { class: "info-icon", tabindex: "0",
										svg {
											fill: "none",
											stroke: "currentColor",
											stroke_width: "2.4",
											view_box: "0 0 24 24",
											circle {
												cx: "12",
												cy: "12",
												r: "10",
											}
											path { d: "M12 16v-4M12 8h.01" }
										}
										span { class: "info-icon__tip",
											"The score a team must reach — and clear, if Win By 2 is on — to win the game."
										}
									}
								}
								select { class: "dropdown", id: "playTo",
									option { value: "11", "11" }
									option { selected: "false", value: "21", "21" }
									option { value: "31", "31" }
								}
							}
						}
					}
					div {
						class: "panel__footer",
						id: "lobbyFooter",
						style: "margin-top: var(--space-4);",
					}
				}
				main { class: "lobby-arena",
					div {
						aria_label: "Seats at the table",
						class: "player-ellipse",
						id: "playerEllipse",
					}
					div { class: "lobby-arena__footer",
						button {
							class: "btn btn--ghost btn--sm",
							id: "addPlayerBtn",
							r#type: "button",
							"+ Add Player"
						}
						span { class: "lobby-arena__count", id: "seatCount", "4 / 8 players" }
					}
					p { class: "lobby-arena__hint", id: "editHint",
						"Click a seat, then click another to swap them."
					}
				}
			}
		}
	}
}
