use {
	crate::{Route, components::*, screens::*},
	dioxus::prelude::*,
	libsetback::*,
};

#[component]
pub fn BrowseGames() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: PANEL_CSS }

		BackBtn { to: Route::Join }
		main { class: "app-shell",
			div { class: "screen",
				div { class: "top-bar", style: "--topbar-max: 60rem;",
					h1 { class: "top-bar__title", "Public Tables" }
					div { class: "top-bar__trailing",
						Btn {
							variant: BtnVariant::Ghost,
							size: BtnSize::Sm,
							svg {
								fill: "none",
								stroke: "currentColor",
								stroke_width: "2",
								style: "width:1em;height:1em;",
								view_box: "0 0 24 24",
								path { d: "M3 12a9 9 0 1 0 3-6.7M3 4v5h5" }
							}
							"\n\t\t\t\t\tRefresh\n\t\t\t\t"
						}
					}
				}
				Panel { size: PanelSize::Wide,
					div { class: "panel__inner",
						div {
							class: "row gap-3",
							style: "flex-wrap: wrap; margin-bottom: var(--space-5);",
							TextInput {
								label: "Search",
								placeholder: "Table name…",
								style: "flex: 2; min-width: 12rem;"
							}
							div {
								class: "field",
								style: "flex: 1; min-width: 9rem;",
								LabeledDropdown {
									label: "Rules",
									options: vec![
										"Standard".into(),
										"Custom".into(),
									]
								}
							}
							div {
								class: "field",
								style: "flex: 1; min-width: 9rem;",
								LabeledDropdown {
									label: "Availability",
									selected_idx: 1,
									options: vec![
										"All tables".into(),
										"Open seats only".into(),
									]
								}
							}
						}
						div { class: "stack gap-3", id: "gameList",
							//TODO replace placeholders with actual data
							GameListItem {
								name: "Ridge's Table",
								std_team_mode: true,
								win_by_2: false,
								negative_scores: false,
								dealer_override: false,
								max_score: 21,
								max_players: 4,
								num_players: 3,
								game_code: "123456",
							}
							GameListItem {
								name: "Front Porch Pitch",
								std_team_mode: false,
								win_by_2: false,
								negative_scores: false,
								dealer_override: false,
								max_score: 11,
								max_players: 4,
								num_players: 3,
								game_code: "123456",
							}
							GameListItem {
								name: "Sunday Slow Table",
								std_team_mode: true,
								win_by_2: false,
								negative_scores: false,
								dealer_override: false,
								max_score: 21,
								max_players: 4,
								num_players: 2,
								game_code: "123456",
							}
							GameListItem {
								name: "High Stakes Hollow",
								std_team_mode: true,
								win_by_2: false,
								negative_scores: false,
								dealer_override: false,
								max_score: 31,
								max_players: 4,
								num_players: 1,
								game_code: "123456",
							}
						}
						p {
							class: "field__hint",
							hidden: "false",
							id: "emptyState",
							style: "text-align:center; padding: var(--space-6) 0;",
							"No tables match your filters."
						}
					}
				}
			}
		}
		ToastRegion {}
	}
}
