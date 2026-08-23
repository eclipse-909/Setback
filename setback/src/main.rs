use {crate::screens::*, dioxus::prelude::*};

mod components;
mod screens;
mod platform;

const BASE_CSS: Asset = asset!("/assets/styling/base.css");
const LAYOUT_CSS: Asset = asset!("/assets/styling/layout.css");
const VARIABLES_CSS: Asset = asset!("/assets/styling/variables.css");

const FAVICON: Asset = asset!("/assets/images/favicon.ico");

fn main() {
	launch(App);
}

#[component]
fn App() -> Element {
	rsx! {
		document::Meta { charset: "UTF-8" }
		document::Link { rel: "icon", href: FAVICON }
		document::Link { rel: "stylesheet", href: VARIABLES_CSS }
		document::Link { rel: "stylesheet", href: BASE_CSS }
		document::Link { rel: "stylesheet", href: LAYOUT_CSS }

		style { r#":root {{ --ui-scale: {UI_SCALE}; }}"# }
		div { class: "table-backdrop"}
		Router::<Route> {}
	}
}

#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[route("/")]
	MainMenu,
	#[route("/browse-games")]
	BrowseGames,
	#[route("/game")]
	Game,
	#[route("/host-game")]
	HostGame,
	#[route("/join")]
	Join,
	#[route("/lobby")]
	Lobby,
	#[route("/multiplayer")]
	Multiplayer,
	#[route("/settings")]
	Settings,
}

/** Everything in the system is sized in rem. Changing --ui-scale (done by
the Settings > Video slider) changes the root font-size, which scales
the entire interface uniformly. */
pub static UI_SCALE: GlobalSignal<f32> = Signal::global(|| 1.);