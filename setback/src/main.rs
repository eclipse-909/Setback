use {crate::screens::*, dioxus::prelude::*};

mod components;
mod screens;

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
		meta { charset: "UTF-8" }
		document::Link { rel: "icon", href: FAVICON }
		document::Link { rel: "stylesheet", href: VARIABLES_CSS }
		document::Link { rel: "stylesheet", href: BASE_CSS }
		document::Link { rel: "stylesheet", href: LAYOUT_CSS }

		div { class: "table-backdrop" }
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
