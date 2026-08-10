use {crate::components::*, dioxus::prelude::*};

#[derive(Props, PartialEq, Clone)]
pub struct GameListItemProps {
	name: String,
	std_team_mode: bool,
	win_by_2: bool,
	negative_scores: bool,
	dealer_override: bool,
	max_score: u8,
	max_players: u8,
	num_players: u8,
	game_code: String,
}

#[component]
pub fn GameListItem(props: GameListItemProps) -> Element {
	let teams = if props.std_team_mode {
		"Standard"
	} else {
		"Custom"
	};
	rsx! {
		document::Link { rel: "stylesheet", href: GAME_LIST_ITEM_CSS }
		document::Link { rel: "stylesheet", href: BUTTON_CSS }

		div { class: "game-list-item",
			span { class: "game-list-item__suit",
				"♠" // TODO use random suit or make suit mean something
			}
			div { class: "game-list-item__info",
				div { class: "game-list-item__name", "{props.name}" }
				div { class: "game-list-item__meta",
					span { "{teams} teams" }
					span { "To {props.max_score}" }
					if props.win_by_2 { span { "Win by 2" } }
					if props.negative_scores { span { "Negative scores" } }
					if props.dealer_override { span { "Dealer override" } }
				}
			}
			span { class: "game-list-item__players", "{props.num_players}/{props.max_players}" }
			Btn { size: BtnSize::Sm, variant: BtnVariant::Primary, "Join" }
		}
	}
}
