use {
	crate::{components::*, *},
	dioxus::prelude::*,
	libsetback::*
};

const GAME_CSS: Asset = asset!("/assets/styling/game.css");

#[component]
pub fn Game() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: CARD_CHIP_CSS }
		document::Link { rel: "stylesheet", href: MODAL_CSS }
		document::Link { rel: "stylesheet", href: PANEL_CSS }
		document::Link { rel: "stylesheet", href: GAME_LIST_ITEM_CSS }
		document::Link { rel: "stylesheet", href: TOAST_CSS }
		document::Link { rel: "stylesheet", href: GAME_CSS }

		div { class: "game-table",
			div { class: "game-hud",
				div { class: "game-hud__score",
					"We: 14\u{a0}\u{a0}|\u{a0}\u{a0}They: 9\u{a0}\u{a0}·\u{a0}\u{a0}Playing to 21"
				}
				ModalBtn {
					modal: "tableMenuModal",
					variant: BtnVariant::Ghost,
					size: BtnSize::Icon,
					svg {
						fill: "none",
						stroke: "currentColor",
						stroke_width: "2",
						view_box: "0 0 24 24",
						circle { cx: "5", cy: "12", r: "1.6" }
						circle { cx: "12", cy: "12", r: "1.6" }
						circle { cx: "19", cy: "12", r: "1.6" }
					}
				}
			}
			div { class: "seats",
				div { class: "seat seat--top",
					span { class: "seat__name", "Deacon" }
					div { class: "seat__avatar", "D" }
					div { class: "seat__hand",
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
					}
				}
				div { class: "seat seat--left is-turn",
					div { class: "seat__avatar", "M" }
					span { class: "seat__name", "Maribel" }
					div { class: "seat__hand",
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
					}
				}
				div { class: "seat seat--right",
					div { class: "seat__avatar", "R" }
					span { class: "seat__name", "Ridge" }
					div { class: "seat__hand",
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
						div { class: "card-chip card-chip--sm card-chip--back" }
					}
				}
				div { class: "trick-area",
					div { class: "card-chip card-chip--red",
						span { class: "card-chip__pip", "♥" }
					}
					div { class: "card-chip",
						span { class: "card-chip__pip", "♠" }
					}
				}
			}
			div { class: "my-hand-bar",
				div { class: "card-chip card-chip--red card-chip--interactive",
					span { class: "card-chip__corner",
						"A"
						br {}
						"♥"
					}
					span { class: "card-chip__pip", "♥" }
				}
				div { class: "card-chip card-chip--interactive",
					span { class: "card-chip__corner",
						"K"
						br {}
						"♣"
					}
					span { class: "card-chip__pip", "♣" }
				}
				div { class: "card-chip card-chip--red card-chip--interactive",
					span { class: "card-chip__corner",
						"Q"
						br {}
						"♦"
					}
					span { class: "card-chip__pip", "♦" }
				}
				div { class: "card-chip card-chip--interactive",
					span { class: "card-chip__corner",
						"J"
						br {}
						"♠"
					}
					span { class: "card-chip__pip", "♠" }
				}
				div { class: "card-chip card-chip--interactive",
					span { class: "card-chip__corner",
						"9"
						br {}
						"♣"
					}
					span { class: "card-chip__pip", "♣" }
				}
				div { class: "card-chip card-chip--red card-chip--interactive",
					span { class: "card-chip__corner",
						"4"
						br {}
						"♥"
					}
					span { class: "card-chip__pip", "♥" }
				}
			}
		}
		div {
			class: "modal-overlay",
			hidden: "false",
			id: "tableMenuModal",
			div {
				aria_labelledby: "tableMenuTitle",
				aria_modal: "true",
				class: "modal panel panel--sm",
				role: "dialog",
				button {
					aria_label: "Close",
					class: "modal__close",
					"data-modal-close": "false",
					svg {
						fill: "none",
						stroke: "currentColor",
						stroke_width: "2",
						view_box: "0 0 24 24",
						path { d: "M18 6L6 18M6 6l12 12" }
					}
				}
				div { class: "panel__inner",
					div { class: "panel__header",
						h2 { class: "panel__title", id: "tableMenuTitle", "Table Menu" }
					}
					div { class: "panel__body stack gap-3",
						a {
							// TODO this needs to open settings in a modal
							class: "btn btn--md btn--full",
							href: "settings.html",
							"Settings"
						}
						LinkBtn {
							to: Route::MainMenu,
							variant: BtnVariant::Danger,
							full: true,
							"Leave Table"
						}
					}
				}
			}
		}
	}
}
