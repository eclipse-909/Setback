use {
	crate::{
		KeybindType, Message, Scene, VolumeType, GameSettings, Host,
		PADDING, UI_ELEMENT_COLOR, TEXT_COLOR, DANGER_COLOR, SUCCESS_COLOR, CARDS_ICON
	},
	iced::{
		alignment, widget::{
			button, checkbox, column, container, image, pick_list, row, scrollable, slider,
			text, text_input, Button, Column, Container, Row, Space, Text,
		},
		advanced::image::Handle,
		Alignment, Color, Element, Length, Renderer, Background, Theme,
	},
	shared::{SetupData, PublicGame, GameFilter}
};

pub fn main_menu() -> Element<'static, Message> {
	// Title
	let title = text("Setback")
		.size(50)
		.style(Color::WHITE)
		.width(Length::Fill)
		.horizontal_alignment(alignment::Horizontal::Center);

	// Logo image
	let logo = container(
		image(Handle::from_bytes(CARDS_ICON))
	)
	.center_x(Length::Fill)
	.center_y(Length::Units(120));

	// Buttons
	let singleplayer_button = button(
		text("Singleplayer").horizontal_alignment(alignment::Horizontal::Center)
	)
	.width(Length::Units(200))
	.padding(10)
	.on_press(Message::SingleplayerPressed);

	let multiplayer_button = button(
		text("Multiplayer").horizontal_alignment(alignment::Horizontal::Center)
	)
	.width(Length::Units(200))
	.padding(10)
	.on_press(Message::MultiplayerPressed);

	let settings_button = button(
		text("Settings").horizontal_alignment(alignment::Horizontal::Center)
	)
	.width(Length::Units(200))
	.padding(10)
	.on_press(Message::SettingsPressed);

	let exit_button = button(
		text("Exit").horizontal_alignment(alignment::Horizontal::Center)
	)
	.width(Length::Units(200))
	.padding(10)
	.style(iced::theme::Button::Destructive)
	.on_press(Message::ExitPressed);

	// Layout
	let content = column![
		title,
		logo,
		Space::with_height(Length::Units(30)),
		singleplayer_button,
		Space::with_height(Length::Units(10)),
		multiplayer_button,
		Space::with_height(Length::Units(10)),
		settings_button,
		Space::with_height(Length::Units(10)),
		exit_button,
	]
	.spacing(5)
	.padding(PADDING)
	.width(Length::Fill)
	.height(Length::Fill)
	.align_items(Alignment::Center);

	container(content)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.into()
}

pub fn settings(game_settings: &GameSettings) -> Element<'static, Message> {
	let back_button = button(text("Back")
		.horizontal_alignment(alignment::Horizontal::Center))
		.on_press(Message::Back)
		.padding(10);

	game_settings.view()
}

pub fn volume_slider(label: &str, value: f32, volume_type: VolumeType) -> Element<'static, Message> {
	let label = text(label).size(16).style(TEXT_COLOR);
	
	let value_text = text(format!("{}%", (value * 100.0) as u32))
		.size(16)
		.style(TEXT_COLOR);
		
	let slider = slider(0.0..=1.0, value, move |v| Message::VolumeChanged(volume_type, v));
		
	row![
		label,
		slider.width(Length::FillPortion(8)),
		value_text.width(Length::FillPortion(2)),
	]
	.spacing(10)
	.align_items(Alignment::Center)
	.width(Length::Fill)
	.into()
}

pub fn keybind_row(label: &str, key_type: KeybindType, value: &str) -> Element<'static, Message> {
	let label = text(label).size(16).style(TEXT_COLOR).width(Length::FillPortion(6));
	
	let keybind_button = button(text(value).horizontal_alignment(alignment::Horizontal::Center))
		.width(Length::FillPortion(3))
		.padding(5)
		.on_press(Message::PromptKeybind);
		
	let reset_button = button(text("Reset").horizontal_alignment(alignment::Horizontal::Center))
		.width(Length::FillPortion(2))
		.padding(5)
		.on_press(Message::ResetKeybind(key_type));
		
	row![
		label,
		keybind_button,
		reset_button,
	]
	.spacing(10)
	.align_items(Alignment::Center)
	.width(Length::Fill)
	.into()
}

pub fn keybind_for(key_type: KeybindType, keybinds: &[(KeybindType, String)]) -> &str {
	for (k, v) in keybinds {
		if *k == key_type {
			return v;
		}
	}
	"Unknown"
}

pub fn multiplayer(public_games: &[PublicGame], filter: &GameFilter) -> Element<'static, Message> {
	let back_button = button(text("Back").horizontal_alignment(alignment::Horizontal::Center))
		.on_press(Message::Back)
		.padding(10);

	let content = column![
		back_button,
		text("Multiplayer").size(30),
		text("Multiplayer game list will be displayed here").size(16),
	]
	.spacing(20)
	.padding(PADDING)
	.width(Length::Fill)
	.align_items(Alignment::Center);

	container(content)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.into()
}

pub fn setup(
	setup_data: &SetupData,
	is_host: bool,
	players: &[(usize, String, bool, u8, (f32, f32))], // (index, name, is_human, team, position)
) -> Element<'static, Message> {
	let back_button = button(text("Back").horizontal_alignment(alignment::Horizontal::Center))
		.on_press(Message::Back)
		.padding(10);

	// Right Panel - Game Settings
	let settings_panel = game_settings_panel(
		setup_data.std_team_mode,
		setup_data.win_by_2,
		setup_data.negative_scores,
		setup_data.dealer_override,
		setup_data.max_score,
		setup_data.max_players,
		is_host,
		setup_data.players.len(),
	);

	// Center Area - Players Ellipse
	let players_area = players_ellipse(players, is_host);

	// Start/Ready Button 
	let start_button = button(
		text(if is_host { "Start" } else { "Ready" })
			.horizontal_alignment(alignment::Horizontal::Center)
	)
	.padding(10)
	.on_press(if is_host { Message::Start } else { Message::Ready(true) });

	// Layout
	let content = row![
		column![
			back_button,
			Space::with_height(Length::Fill),
		].width(Length::FillPortion(1)),
		
		players_area.width(Length::FillPortion(8)),
		
		column![
			settings_panel,
			Space::with_height(Length::Fill),
			start_button,
			Space::with_height(Length::Units(40)),
		].width(Length::FillPortion(3)),
	]
	.padding(PADDING)
	.width(Length::Fill)
	.height(Length::Fill);

	container(content)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.into()
}

fn game_settings_panel(
	std_team_mode: bool,
	win_by_2: bool,
	negative_scores: bool,
	dealer_override: bool,
	max_score: u8,
	max_players: u8,
	is_enabled: bool,
	player_count: usize,
) -> Element<'static, Message> {
	let header = text("Settings")
		.size(24)
		.style(TEXT_COLOR);

	let team_mode_row = row![
		text("Team Mode:").size(16).style(TEXT_COLOR),
		pick_list(
			vec!["Standard", "Custom"],
			Some("Standard"),
			|s| Message::TeamModeChanged(s == "Standard"),
		)
		.style(if is_enabled { 
			|theme: &Theme| pick_list::Appearance::default()
		} else { 
			|theme: &Theme| pick_list::Appearance {
				text_color: theme.palette().text.with_alpha(0.5),
				..pick_list::Appearance::default()
			}
		}),
	]
	.spacing(10)
	.align_items(Alignment::Center);

	let win_by_2_checkbox = checkbox(
		"Win by 2",
		win_by_2
	)
	.on_toggle(|on| Message::WinBy2Changed(on))
	.style(if is_enabled {
		|theme: &Theme| checkbox::Appearance::default()
	} else {
		|theme: &Theme| checkbox::Appearance {
			text_color: Some(theme.palette().text.with_alpha(0.5)),
			..checkbox::Appearance::default()
		}
	});

	let negative_scores_checkbox = checkbox(
		"Negative Scores",
		negative_scores
	)
	.on_toggle(|on| Message::NegativeScoresChanged(on))
	.style(if is_enabled {
		|theme: &Theme| checkbox::Appearance::default()
	} else {
		|theme: &Theme| checkbox::Appearance {
			text_color: Some(theme.palette().text.with_alpha(0.5)),
			..checkbox::Appearance::default()
		}
	});

	let dealer_override_checkbox = checkbox(
		"Dealer Override",
		dealer_override
	)
	.on_toggle(|on| Message::DealerOverrideChanged(on))
	.style(if is_enabled {
		|theme: &Theme| checkbox::Appearance::default()
	} else {
		|theme: &Theme| checkbox::Appearance {
			text_color: Some(theme.palette().text.with_alpha(0.5)),
			..checkbox::Appearance::default()
		}
	});

	let max_score_row = row![
		text("Max Score:").size(16).style(TEXT_COLOR),
		// In a full implementation, this would be a number input component
		text(max_score.to_string()).size(16).style(TEXT_COLOR),
	]
	.spacing(10)
	.align_items(Alignment::Center);

	let max_players_row = row![
		text("Max Players:").size(16).style(TEXT_COLOR),
		// In a full implementation, this would be a number input component
		text(max_players.to_string()).size(16).style(TEXT_COLOR),
	]
	.spacing(10)
	.align_items(Alignment::Center);

	let add_player_button = button(
		text("Add Player").horizontal_alignment(alignment::Horizontal::Center)
	)
	.width(Length::Fill)
	.padding(10)
	.on_press(Message::AddPlayer)
	.style(if is_enabled && (player_count as u8) < max_players {
		iced::theme::Button::Primary 
	} else { 
		iced::theme::Button::Secondary 
	});

	column![
		header,
		Space::with_height(Length::Units(20)),
		team_mode_row,
		Space::with_height(Length::Units(10)),
		win_by_2_checkbox,
		Space::with_height(Length::Units(10)),
		negative_scores_checkbox,
		Space::with_height(Length::Units(10)),
		dealer_override_checkbox,
		Space::with_height(Length::Units(10)),
		max_score_row,
		Space::with_height(Length::Units(10)),
		max_players_row,
		Space::with_height(Length::Units(20)),
		add_player_button,
	]
	.spacing(5)
	.padding(10)
	.into()
}

fn players_ellipse(
	players: &[(usize, String, bool, u8, (f32, f32))], // (index, name, is_human, team, position)
	is_host: bool,
) -> Element<'static, Message> {
	// In a real implementation, this would position player components on an ellipse
	// For now, just create a simple list of players
	let player_components: Vec<Element<_>> = players
		.iter()
		.map(|(index, name, is_human, team, position)| {
			// Create a player component
			let avatar = text(if *is_human { "👤" } else { "🤖" })
				.size(40)
				.style(TEXT_COLOR);
				
			let name_text = text(name)
				.size(16)
				.style(TEXT_COLOR);
				
			let team_text = text(format!("Team {}", team))
				.size(14)
				.style(TEXT_COLOR);
				
			let remove_button = if is_host {
				button(text("X").size(16))
					.padding(5)
					.on_press(Message::RemovePlayer(*index))
					.style(iced::theme::Button::Destructive)
			} else {
				button(text("")).into()
			};
			
			container(
				column![
					row![
						Space::with_width(Length::Fill),
						remove_button,
					],
					avatar,
					name_text,
					team_text,
				]
				.spacing(5)
				.align_items(Alignment::Center)
			)
			.padding(10)
			.center_x(Length::Units(120))
			.center_y(Length::Units(150))
			.style(|theme: &Theme| container::Appearance {
				background: Some(Background::Color(theme.palette().background.with_alpha(0.7))),
				border_radius: 5.0.into(),
				border_width: 1.0,
				border_color: theme.palette().text,
				..Default::default()
			})
			.into()
		})
		.collect();

	// In a real implementation, these would be positioned based on the ellipse formula
	// For now, just put them in a row
	row(player_components)
		.spacing(20)
		.align_items(Alignment::Center)
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

pub fn game(
	round: u32,
	scores: &[(u8, i32)], // (team, score)
	trump_suit: Option<&str>,
	lead_suit: Option<&str>,
	highest_bid: u8,
	bidder_name: &str,
	pile_owner: &str,
	active_player_index: u8,
	local_player_index: u8,
	hand: &[String], // Card representations
	center_pile: &[String], // Card representations
	player_card_counts: &[(u8, u8)], // (player_index, card_count)
) -> Element<'static, Message> {
	// Top Bar with Settings Gear
	let settings_button = button(text("⚙").size(20))
		.padding(10)
		.on_press(Message::SettingsPressed);

	// Game Info Bar (Bottom)
	let round_info = text(format!("Round {}", round))
		.size(16)
		.style(TEXT_COLOR);

	let scores_text = scores
		.iter()
		.map(|(team, score)| format!("Team {}: {}", team, score))
		.collect::<Vec<_>>()
		.join(" - ");
		
	let scores_info = text(scores_text)
		.size(16)
		.style(TEXT_COLOR);

	let trump_info = text(format!("Trump: {}", trump_suit.unwrap_or("None")))
		.size(16)
		.style(TEXT_COLOR);

	let lead_info = text(format!("Lead: {}", lead_suit.unwrap_or("None")))
		.size(16)
		.style(TEXT_COLOR);

	let bid_info = text(format!("Bid: {} by {}", highest_bid, bidder_name))
		.size(16)
		.style(TEXT_COLOR);

	let pile_owner_info = text(format!("Pile: {}", pile_owner))
		.size(16)
		.style(TEXT_COLOR);

	let info_bar = row![
		round_info,
		Space::with_width(Length::Fill),
		scores_info,
		Space::with_width(Length::Fill),
		column![
			row![trump_info, Space::with_width(Length::Units(10)), lead_info],
			row![bid_info, Space::with_width(Length::Units(10)), pile_owner_info],
		],
	]
	.spacing(10)
	.padding(10)
	.width(Length::Fill)
	.align_items(Alignment::Center);

	// Main Game Area (Players and Cards)
	// This would be a complex layout with the ellipse of players and cards in the center
	// For now, just display a simple placeholder

	let player_cards = text("Player Cards would be displayed here")
		.size(16)
		.style(TEXT_COLOR);

	let center_pile_text = text("Center Pile would be displayed here")
		.size(16)
		.style(TEXT_COLOR);

	let game_area = container(
		column![
			center_pile_text,
			Space::with_height(Length::Units(50)),
			player_cards,
		]
		.spacing(20)
		.align_items(Alignment::Center)
	)
	.center_x(Length::Fill)
	.center_y(Length::Fill);

	// Full Layout
	let content = column![
		row![
			settings_button,
			Space::with_width(Length::Fill),
		]
		.padding(10),
		
		game_area,
		
		info_bar,
	]
	.width(Length::Fill)
	.height(Length::Fill);

	container(content)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.into()
}