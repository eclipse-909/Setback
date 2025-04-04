use {
	crate::{
		Message, CARD_MOVE_DURATION, PLAYER_MOVE_DURATION, TEXT_COLOR,
		calculate_player_position
	},
	iced::{
		alignment, advanced::{
			layout::{self, Layout},
			renderer,
			widget::{self, Widget},
		},
		widget::{
			button, column, container, row, text, Button, Column, Container, Row, Space,
		},
		Alignment, Background, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
	},
	shared::{Card, Suit},
	std::time::{Duration, Instant},
};

/// A visual representation of a playing card
#[derive(Debug, Clone)]
pub struct CardComponent {
	card: Card,
	dragging: bool,
	draggable: bool,
	playable: bool,
	position: Point,
	target: Option<Point>,
	size: f32,
	start_time: Option<Instant>,
}

impl CardComponent {
	pub fn new(card: Card, position: Point, size: f32) -> Self {
		Self {
			card,
			dragging: false,
			draggable: true,
			playable: true,
			position,
			target: None,
			size,
			start_time: None,
		}
	}

	pub fn draggable(mut self, draggable: bool) -> Self {
		self.draggable = draggable;
		self
	}

	pub fn playable(mut self, playable: bool) -> Self {
		self.playable = playable;
		self
	}

	pub fn animate_to(mut self, target: Point) -> Self {
		self.target = Some(target);
		self.start_time = Some(Instant::now());
		self
	}

	pub fn update_position(&mut self, now: Instant) -> bool {
		if let (Some(target), Some(start_time)) = (self.target, self.start_time) {
			let elapsed = now.duration_since(start_time);
			
			if elapsed >= CARD_MOVE_DURATION {
				self.position = target;
				self.target = None;
				self.start_time = None;
				false
			} else {
				let progress = elapsed.as_secs_f32() / CARD_MOVE_DURATION.as_secs_f32();
				let eased_progress = 1.0 - (1.0 - progress).powi(3); // Ease out cubic
				
				self.position.x = self.position.x + (target.x - self.position.x) * eased_progress;
				self.position.y = self.position.y + (target.y - self.position.y) * eased_progress;
				true
			}
		} else {
			false
		}
	}

	pub fn set_dragging(&mut self, dragging: bool) {
		self.dragging = dragging;
	}

	pub fn view(&self) -> Element<Message> {
		// Simplified placeholder card display
		let card_text = match self.card.suit {
			Suit::Clubs => "♣",
			Suit::Diamonds => "♦",
			Suit::Hearts => "♥",
			Suit::Spades => "♠",
		};
		
		let value_text = match self.card.number {
			11 => "J",
			12 => "Q",
			13 => "K",
			14 => "A",
			v => v.to_string().as_str(),
		};
		
		let card_color = match self.card.suit {
			Suit::Hearts | Suit::Diamonds => Color::from_rgb(0.8, 0.0, 0.0),
			_ => Color::BLACK,
		};
		
		let opacity = if self.playable { 1.0 } else { 0.5 };
		
		let card_display = column![
			text(card_text)
				.size(self.size as u16 / 2)
				.style(card_color.with_alpha(opacity)),
			text(value_text)
				.size(self.size as u16 / 3)
				.style(card_color.with_alpha(opacity)),
		]
		.align_items(Alignment::Center)
		.spacing(5);
		
		let card_container = container(card_display)
			.center_x(Length::Units(self.size as u16))
			.center_y(Length::Units((self.size * 1.4) as u16))
			.style(|theme: &Theme| {
				let shadow_offset = if self.dragging { 8.0 } else { 2.0 };
				let border_color = if !self.playable {
					Color::from_rgb(0.6, 0.6, 0.6)
				} else if self.dragging {
					Color::from_rgb(0.3, 0.6, 0.3)
				} else {
					Color::from_rgb(0.1, 0.1, 0.1)
				};
				
				container::Appearance {
					background: Some(Background::Color(Color::WHITE)),
					border_radius: 5.0.into(),
					border_width: 1.0,
					border_color,
					shadow_offset: iced::Vector::new(shadow_offset, shadow_offset),
					..Default::default()
				}
			});
			
		if self.draggable && self.playable {
			// TODO: In a full implementation, this would use proper drag-drop
			card_container.into()
		} else {
			card_container.into()
		}
	}
}

/// A visual representation of a player in the game
#[derive(Debug, Clone)]
pub struct PlayerComponent {
	index: usize,
	name: String,
	is_human: bool,
	team: u8,
	local: bool,
	active: bool,
	position: Point,
	target: Option<Point>,
	start_time: Option<Instant>,
	cards_count: u8,
	ready: bool,
}

impl PlayerComponent {
	pub fn new(
		index: usize,
		name: String,
		is_human: bool,
		team: u8,
		local: bool,
		position: Point,
	) -> Self {
		Self {
			index,
			name,
			is_human,
			team,
			local,
			active: false,
			position,
			target: None,
			start_time: None,
			cards_count: 0,
			ready: false,
		}
	}

	pub fn active(mut self, active: bool) -> Self {
		self.active = active;
		self
	}

	pub fn cards_count(mut self, count: u8) -> Self {
		self.cards_count = count;
		self
	}

	pub fn ready(mut self, ready: bool) -> Self {
		self.ready = ready;
		self
	}

	pub fn animate_to(mut self, target: Point) -> Self {
		self.target = Some(target);
		self.start_time = Some(Instant::now());
		self
	}

	pub fn update_position(&mut self, now: Instant) -> bool {
		if let (Some(target), Some(start_time)) = (self.target, self.start_time) {
			let elapsed = now.duration_since(start_time);
			
			if elapsed >= PLAYER_MOVE_DURATION {
				self.position = target;
				self.target = None;
				self.start_time = None;
				false
			} else {
				let progress = elapsed.as_secs_f32() / PLAYER_MOVE_DURATION.as_secs_f32();
				let eased_progress = 1.0 - (1.0 - progress).powi(3); // Ease out cubic
				
				self.position.x = self.position.x + (target.x - self.position.x) * eased_progress;
				self.position.y = self.position.y + (target.y - self.position.y) * eased_progress;
				true
			}
		} else {
			false
		}
	}

	pub fn view(&self) -> Element<Message> {
		// This would use actual icons in a full implementation
		let avatar = text(if self.is_human { "👤" } else { "🤖" })
			.size(40)
			.style(TEXT_COLOR);
			
		let name_text = text(&self.name)
			.size(16)
			.style(TEXT_COLOR);
			
		// Team chip with color based on team number
		let team_colors = [
			Color::from_rgb(0.2, 0.4, 0.8),  // Blue
			Color::from_rgb(0.8, 0.2, 0.2),  // Red
			Color::from_rgb(0.2, 0.7, 0.2),  // Green
			Color::from_rgb(0.8, 0.7, 0.2),  // Yellow
		];
		
		let team_color = team_colors[(self.team as usize) % team_colors.len()];
		
		let team_chip = container(
			text(format!("Team {}", self.team))
				.size(12)
				.style(Color::WHITE)
		)
		.padding(4)
		.style(|_theme: &Theme| container::Appearance {
			background: Some(Background::Color(team_color)),
			border_radius: 12.0.into(),
			..Default::default()
		});
		
		// Ready indicator
		let ready_indicator = if self.ready {
			container(text("✓").size(16).style(Color::WHITE))
				.padding(4)
				.style(|_theme: &Theme| container::Appearance {
					background: Some(Background::Color(Color::from_rgb(0.2, 0.7, 0.2))),
					border_radius: 12.0.into(),
					..Default::default()
				})
				.into()
		} else {
			Space::with_height(Length::Units(24)).into()
		};
		
		// Optional card count display for game scene
		let cards_display = if self.cards_count > 0 {
			container(
				text(format!("Cards: {}", self.cards_count))
					.size(12)
					.style(TEXT_COLOR)
			)
			.padding(4)
			.into()
		} else {
			Space::with_height(Length::Units(20)).into()
		};
		
		let active_border_style = |theme: &Theme| {
			let color = if self.is_local {
				Color::from_rgb(0.2, 0.8, 0.2) // Green for local player
			} else {
				Color::from_rgb(0.9, 0.7, 0.0) // Yellow for other players
			};
			
			container::Appearance {
				background: Some(Background::Color(Color::from_rgba(0.1, 0.1, 0.1, 0.5))),
				border_radius: 5.0.into(),
				border_width: 2.0,
				border_color: color,
				shadow_offset: iced::Vector::new(0.0, 0.0),
				..Default::default()
			}
		};
		
		let container_style = if self.active {
			active_border_style
		} else {
			|theme: &Theme| container::Appearance {
				background: Some(Background::Color(theme.palette().background.with_alpha(0.7))),
				border_radius: 5.0.into(),
				border_width: 1.0,
				border_color: theme.palette().text,
				..Default::default()
			}
		};
		
		container(
			column![
				avatar,
				name_text,
				team_chip,
				ready_indicator,
				cards_display,
			]
			.spacing(5)
			.align_items(Alignment::Center)
		)
		.padding(10)
		.center_x(Length::Units(120))
		.center_y(Length::Units(180))
		.style(container_style)
		.into()
	}
}

/// Creates a modal overlay with a customizable content
pub fn modal<'a, Message>(
	content: impl Into<Element<'a, Message>>,
	on_close: Message,
) -> Container<'a, Message>
where
	Message: 'a + Clone,
{
	let content = container(content)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.style(|theme: &Theme| container::Appearance {
			background: Some(Background::Color(theme.palette().background)),
			border_radius: 5.0.into(),
			border_width: 1.0,
			border_color: theme.palette().text,
			..Default::default()
		});

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_theme: &Theme| container::Appearance {
			background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.7))),
			..Default::default()
		})
		// TODO: In full implementation, this would have a click handler
		// to close when clicking outside the modal
}

/// Creates a stylized suit icon
pub fn suit_icon(suit: Option<Suit>, size: u16) -> Element<'static, Message> {
	let (symbol, color) = match suit {
		Some(Suit::Clubs) => ("♣", Color::BLACK),
		Some(Suit::Diamonds) => ("♦", Color::from_rgb(0.8, 0.0, 0.0)),
		Some(Suit::Hearts) => ("♥", Color::from_rgb(0.8, 0.0, 0.0)),
		Some(Suit::Spades) => ("♠", Color::BLACK),
		None => ("?", Color::from_rgb(0.5, 0.5, 0.5)),
	};
	
	text(symbol)
		.size(size)
		.style(color)
		.horizontal_alignment(alignment::Horizontal::Center)
		.vertical_alignment(alignment::Vertical::Center)
		.into()
}

/// Creates a layout positioning players in a circular/elliptical arrangement
pub struct PlayersCircle<'a, Message> {
	players: Vec<Element<'a, Message>>,
}

impl<'a, Message> PlayersCircle<'a, Message> {
	pub fn new() -> Self {
		Self {
			players: Vec::new(),
		}
	}

	pub fn push(mut self, player: Element<'a, Message>) -> Self {
		self.players.push(player);
		self
	}
}

impl<'a, Message: 'a> Widget<Message, Renderer> for PlayersCircle<'a, Message> {
	fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
		let size = limits.max();
		
		let player_size = Size::new(120.0, 180.0);
		let center = Point::new(size.width / 2.0, size.height / 2.0);
		
		let mut nodes = Vec::with_capacity(self.players.len());
		
		for (i, player) in self.players.iter().enumerate() {
			let player_layout = player.layout(renderer, &layout::Limits::new(
				Size::ZERO,
				player_size,
			));
			
			let (x, y) = calculate_player_position(i, self.players.len());
			let position = Point::new(
				center.x + x - player_layout.size().width / 2.0,
				center.y + y - player_layout.size().height / 2.0,
			);
			
			nodes.push(layout::Node::with_children(
				player_layout.size(),
				vec![player_layout],
			).move_to(position));
		}

		layout::Node::with_children(size, nodes)
	}

	fn draw(
		&self,
		state: &widget::Tree,
		renderer: &mut Renderer,
		theme: &iced::Theme,
		style: &renderer::Style,
		layout: Layout<'_>,
		cursor: iced::mouse::Cursor,
		viewport: &Rectangle,
	) {
		let children = layout.children();
		
		for (i, (player, layout)) in self.players.iter().zip(children).enumerate() {
			player.draw(
				&state.children[i],
				renderer,
				theme,
				style,
				layout,
				cursor,
				viewport,
			);
		}
	}
	
	fn size_hint(&self) -> Size<Length> {
		self.size()
	}
	
	fn tag(&self) -> widget::tree::Tag {
		widget::tree::Tag::stateless()
	}
	
	fn state(&self) -> widget::tree::State {
		widget::tree::State::None
	}
	
	fn children(&self) -> Vec<widget::Tree> {
		Vec::new()
	}
	
	fn diff(&self, _tree: &mut widget::Tree) {}
	
	fn operate(
		&self,
		_state: &mut widget::Tree,
		_layout: Layout<'_>,
		_renderer: &Renderer,
		_operation: &mut dyn widget::Operation,
	) {
	}
	
	fn on_event(
		&mut self,
		_state: &mut widget::Tree,
		_event: iced::Event,
		_layout: Layout<'_>,
		_cursor: iced::advanced::mouse::Cursor,
		_renderer: &Renderer,
		_clipboard: &mut dyn iced::advanced::Clipboard,
		_shell: &mut iced::advanced::Shell<'_, Message>,
		_viewport: &Rectangle,
	) -> iced::advanced::graphics::core::event::Status {
		iced::advanced::graphics::core::event::Status::Ignored
	}
	
	fn mouse_interaction(
		&self,
		_state: &widget::Tree,
		_layout: Layout<'_>,
		_cursor: iced::advanced::mouse::Cursor,
		_viewport: &Rectangle,
		_renderer: &Renderer,
	) -> iced::advanced::mouse::Interaction {
		iced::advanced::mouse::Interaction::None
	}
	
	fn overlay<'a>(
		&'a mut self,
		_state: &'a mut widget::Tree,
		_layout: Layout<'_>,
		_renderer: &Renderer,
		_translation: iced::Vector,
	) -> Option<iced::advanced::overlay::Element<'a, Message, Renderer, Renderer>> {
		None
	}
		
	fn size(&self) -> Size<Length> {
		Size::new(Length::Fill, Length::Fill)
	}
}

impl<'a, Message: 'a> From<PlayersCircle<'a, Message>> for Element<'a, Message> {
	fn from(players_circle: PlayersCircle<'a, Message>) -> Self {
		Element::new(players_circle)
	}
}