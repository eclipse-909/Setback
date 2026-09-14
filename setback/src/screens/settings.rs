use {
	crate::{Route, components::*},
	dioxus::prelude::*,
	setback::*,
};

#[component]
pub fn Settings() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: SETTINGS_CSS }

		BackBtn { to: Route::MainMenu }
		main { class: "app-shell",
			div { class: "screen",
				SettingsPanel {
					in_game: false
				}
			}
		}
		ToastRegion {}
	}
}
