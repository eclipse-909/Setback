use {
	dioxus::prelude::*,
	components::*,
};

mod components;

const VARIABLES_CSS: Asset = asset!("/assets/styling/variables.css");
const BASE_CSS: Asset = asset!("/assets/styling/base.css");
const LAYOUT_CSS: Asset = asset!("/assets/styling/layout.css");
const FAVICON: Asset = asset!("/assets/favicon.ico");

fn main() {
	launch(App);
}

#[component]
fn App() -> Element {
	rsx! {
		document::Link { rel: "icon", href: FAVICON }
		document::Link { rel: "stylesheet", href: VARIABLES_CSS }
		document::Link { rel: "stylesheet", href: BASE_CSS }
		document::Link { rel: "stylesheet", href: LAYOUT_CSS }
		
		div { class: "table-backdrop" }
		main { class: "app-shell",
			div { class: "screen",
				Router::<Route> {}
			}
		}
	}
}

#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[route("/")]
	MainMenu,
	// #[route("/settings")]
	// Settings,
}