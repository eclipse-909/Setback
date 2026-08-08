use {
	dioxus::prelude::*,
	libsetback::*,
};

const BUTTON_CSS: Asset = asset!("/assets/styling/button.css");
const CARD_CHIP_CSS: Asset = asset!("/assets/styling/card-chip.css");

#[component]
pub fn MainMenu() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: BUTTON_CSS }
		document::Link { rel: "stylesheet", href: CARD_CHIP_CSS }
		
		div { class: "logo-mark",
			div { aria_hidden: "true", class: "logo-mark__cards",
				div { class: "card-chip card-chip--red",
					span { class: "card-chip__pip", "♥" }
				}
				div { class: "card-chip",
					span { class: "card-chip__pip", "♠" }
				}
				div { class: "card-chip card-chip--red",
					span { class: "card-chip__pip", "♦" }
				}
			}
			h1 { class: "logo-mark__title", "Setback" }
			p { class: "logo-mark__tagline", "Auction · Pitch · Set" }
		}
		nav { aria_label: "Main menu", class: "menu-stack",
			a {
				class: "btn btn--primary btn--lg btn--full",
				href: "screens/lobby.html?context=single&role=host",
				"Singleplayer"
			}
			a {
				class: "btn btn--primary btn--lg btn--full",
				href: "screens/multiplayer.html",
				"Multiplayer"
			}
			a {
				class: "btn btn--md btn--full",
				href: "screens/settings.html",
				"Settings"
			}
			button {
				class: "btn btn--ghost btn--md btn--full",
				id: "exitBtn",
				r#type: "button",
				"Exit Game"
			}
		}
		p { class: "footer-note", "Setback · v{pkg_version!()}" }
	}
}