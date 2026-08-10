use {
	crate::{components::*, *},
	dioxus::prelude::*,
	libsetback::*,
};

#[component]
pub fn MainMenu() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: CARD_CHIP_CSS }

		main { class: "app-shell",
			div { class: "screen",
				div { class: "logo-mark",
					div { aria_hidden: "true", class: "logo-mark__cards",
						CardChip { suit: Suit::Hearts, face_up: true }
						CardChip { suit: Suit::Spades, face_up: true }
						CardChip { suit: Suit::Diamonds, face_up: true }
					}
					h1 { class: "logo-mark__title", "Setback" }
					p { class: "logo-mark__tagline", "Auction · Pitch · Set" }
				}
				nav { aria_label: "Main menu", class: "menu-stack",
					LinkBtn {
						variant: BtnVariant::Primary,
						size: BtnSize::Lg,
						full: true,
						to: Route::Lobby,
						"Singleplayer"
					}
					LinkBtn {
						variant: BtnVariant::Primary,
						size: BtnSize::Lg,
						full: true,
						to: Route::Multiplayer,
						"Multiplayer"
					}
					LinkBtn {
						full: true,
						to: Route::Settings,
						"Settings"
					}
					if cfg!(not(target_family = "wasm")) {
						Btn {
							variant: BtnVariant::Ghost,
							full: true,
							onclick: move |_| {
								#[cfg(not(target_family = "wasm"))]
								dioxus::desktop::window().close();
							},
							"Exit Game"
						}
					}
				}
				p { class: "footer-note", "Setback · v{pkg_version!()}" }
			}
		}
	}
}
