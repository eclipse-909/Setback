use dioxus::prelude::*;

const PLAYER_NODE_CSS: Asset = asset!("/assets/styling/player-node.css");

#[derive(Props, PartialEq, Clone)]
pub struct PlayerNodeProps {
	node_type: PlayerNodeType,
	editable: bool,
	removable: bool,
}

#[derive(PartialEq, Clone)]
pub enum PlayerNodeType {
	Player { name: String, role: PlayerNodeRole, team: u8, is_you: bool },
	Empty
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PlayerNodeRole { Host, Player, Bot }

#[component]
pub fn PlayerNode(props: PlayerNodeProps) -> Element {
	let rm_btn = rsx! {
		button { aria_label: "Remove seat", class: "player-node__remove",
			svg {
				fill: "none",
				stroke: "currentColor",
				stroke_width: "3",
				view_box: "0 0 24 24",
				path { d: "M18 6L6 18M6 6l12 12" }
			}
		}
	};
	let node = match props.node_type {
		PlayerNodeType::Player { name, role, team, is_you } => {
			let editable = if props.editable { "is-editable" } else { "" };
			let role = format!("{role:?}");
			let is_you = if is_you { "is-you" } else { "" };
			rsx! {
				div { class: "player-node {is_you} {editable}",
					div { class: "player-node__avatar", "{name.chars().nth(0).unwrap()}" }
					div { class: "player-node__name", "{name}" }
					div { class: "player-node__tags",
						span { class: "player-node__tag player-node__tag--{role.to_lowercase()}", "{role}" }
					}
					span { class: "player-node__team player-node__team--{team}", "Team {team}" }
					if props.removable {{rm_btn}}
				}
			}
		},
		PlayerNodeType::Empty => rsx! {
			div { class: "player-node is-empty",
				div { class: "player-node__avatar", "?" }
				div { class: "player-node__name", "Open Seat" }
				if props.removable {{rm_btn}}
			}
		}
	};
	
	rsx! {
		document::Link { rel: "stylesheet", href: PLAYER_NODE_CSS }
		
		{node}
	}
}
