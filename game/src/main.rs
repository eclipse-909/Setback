#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	crate::views::{volume_slider, keybind_row},
	env_logger::{Builder, Env},
	iced::{
		futures::{channel::mpsc, Stream, StreamExt},
		keyboard::{self, key::Named, on_key_pressed, Key, Modifiers},
		widget::{button, column, container, row, scrollable, text, text_input, Column, Container, Space},
		alignment, application, color, event, executor, font, mouse, overlay, padding, stream, window, Alignment, Application, Color, Command, Element, Length, Renderer, Settings, Subscription, Theme
	},
	iced_aw::graphics::icons::{CHECKMARK, REGULAR_GEAR_DOT, REGULAR_X_MARK, VOLUME_HIGH},
	image::ImageFormat,
	log::LevelFilter,
	shared::{Card, Error, Game, GameEvent, GameRuntime, PlayerData, PlayerType, Protocol, PublicGame, RoundScores, ServerConnection, SetbackConnection, SetupData, Ssp, Suit, GameFilter},
	std::{
		collections::HashMap,
		env,
		f32::consts::PI,
		fs::{self, File},
		path::{Path, PathBuf},
		sync::atomic::{AtomicBool, Ordering},
		time::{Duration, Instant}
	},
	tokio::sync::mpsc::{self, Receiver, Sender}
};

mod views;
mod components;

// Constants
const LOG: bool = false;
const PROJ_DIR: &str = "./";

// Application theme colors
const BACKGROUND_COLOR: Color = Color::from_rgb8(0x11, 0x44, 0x11);
const TEXT_COLOR: Color = Color::from_rgb8(0xCC, 0xFF, 0xCC);
const UI_ELEMENT_COLOR: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
const DANGER_COLOR: Color = Color::from_rgb8(0xCC, 0x00, 0x00);
const SUCCESS_COLOR: Color = Color::from_rgb8(0x32, 0x47, 0x31);

// Layout constants
const PADDING: f32 = 40.0;
const ELLIPSE_RADIUS: f32 = 525.0;
const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

// Animation constants
const CARD_MOVE_DURATION: Duration = Duration::from_millis(300);
const PLAYER_MOVE_DURATION: Duration = Duration::from_millis(500);
const SCORE_UPDATE_DURATION: Duration = Duration::from_millis(800);
const FADE_DURATION: Duration = Duration::from_millis(200);

// Asset paths - would be embedded in a real implementation
// Icons and images
const CARDS_ICON: &[u8] = include_bytes!("../assets/images/cardsIcon.png");
const PERSON_ICON: &[u8] = include_bytes!("../assets/images/personIcon.png");
const BOT_ICON: &[u8] = include_bytes!("../assets/images/botIcon.png");
const GEAR: &[u8] = include_bytes!("../assets/images/gear.png");

fn main() -> iced::Result {
	//logger
	if LOG {
		#[cfg(debug_assertions)]
		{
			// In debug builds, use default behavior (stderr)
			Builder::from_env(Env::default().default_filter_or("debug"))
				.format_timestamp_micros()
				.init();
		}
		#[cfg(not(debug_assertions))]
		{
			// In release builds, log to file
			Builder::from_env(Env::default().default_filter_or("debug"))
				.format_timestamp_micros()
				.target(env_logger::Target::Pipe(Box::new(File::create(PROJ_DIR.join("log.txt")).unwrap())))
				.init();
		}
	}

	// Create Iced application
	iced::application(
		"Setback",
		Setback::update,
		Setback::view
	)
		.window(window::Settings {
			icon: Some(window::icon::from_file_data(CARDS_ICON, Some(ImageFormat::Png)).unwrap()),
			..Default::default()
		})
		.subscription(Setback::subscription)
		.theme(Setback::theme)
		.centered()
		.antialiasing(true)
		.run()
}

#[derive(Default)]
struct Setback {
	scene: Scene,
	host: Host,
	connection: Option<Sender<ConnectionEvent>>,
	local_player_index: u8,
	settings: GameSettings,
	animation_state: AnimationState,
	error: Option<String>
}

#[repr(u8)]
#[derive(Debug, Clone)]
enum Message {
	// Scene navigation
	TransitionTo(Scene),
	Back,
	
	// Main menu actions
	SingleplayerPressed,
	MultiplayerPressed,
	PromptCredentials,
	CreateGame{public: bool},
	JoinGame{code: u32},
	ExitPressed,
	
	// Settings actions
	VolumeChanged(VolumeType, f32),
	PromptKeybind,
	CancelKeybind,
	KeybindChanged(KeybindType, Key, Modifiers),
	ResetKeybind(KeybindType),
	
	// Game setup actions
	TeamModeChanged{is_std: bool},
	WinBy2Changed(bool),
	NegativeScoresChanged(bool),
	DealerOverrideChanged(bool),
	MaxScoreChanged(u8),
	MaxPlayersChanged(u8),
	AddPlayer,
	RemovePlayer(usize),
	UpdatePlayerTeam(usize, u8),
	UpdatePlayerHuman(usize, bool),
	DragPlayer(usize, f32, f32),
	DropPlayer(usize),
	Ready(bool),
	Start,
	
	// Network actions
	ConnectionEvent(ConnectionEvent),
	
	// Game actions
	GameEvent(GameEvent),
	MakeBid(u8),
	PlayCard(Card),
	PlayCardIndex(u8),
	CardDragged { id: usize, position: (f32, f32) },
	CardDropped { id: usize },
	ToggleSettings,
	
	// Animation and timing
	AnimationTick(Instant),
	
	// Error handling
	ErrorOccurred(String),
	DismissError,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Scene {
	#[default]
	MainMenu,
	Multiplayer(Vec<PublicGame>, GameFilter),
	Settings,
	Setup(SetupData),
	SingleplayerGame(GameRuntime),
	MultiplayerGame(Game)
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Host {
	#[default]
	Singleplayer,
	Host,
	Client
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeType {
	Master,
	Music,
	UI
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeybindType {
	Pass,
	Bid2,
	Bid3,
	Bid4,
	Bid5,
	PlayCard1,
	PlayCard2,
	PlayCard3,
	PlayCard4,
	PlayCard5,
	PlayCard6,
	ToggleSettings
}

impl KeybindType {
	const fn default_keybind(self) -> (Key, Modifiers) {
		match self {
			KeybindType::Pass => (Key::Character('0'), Modifiers::empty()),
			KeybindType::Bid2 => (Key::Character('2'), Modifiers::empty()),
			KeybindType::Bid3 => (Key::Character('3'), Modifiers::empty()),
			KeybindType::Bid4 => (Key::Character('4'), Modifiers::empty()),
			KeybindType::Bid5 => (Key::Character('5'), Modifiers::empty()),
			KeybindType::PlayCard1 => (Key::Character('1'), Modifiers::empty()),
			KeybindType::PlayCard2 => (Key::Character('2'), Modifiers::empty()),
			KeybindType::PlayCard3 => (Key::Character('3'), Modifiers::empty()),
			KeybindType::PlayCard4 => (Key::Character('4'), Modifiers::empty()),
			KeybindType::PlayCard5 => (Key::Character('5'), Modifiers::empty()),
			KeybindType::PlayCard6 => (Key::Character('6'), Modifiers::empty()),
			KeybindType::ToggleSettings => (Key::Named(Named::Escape), Modifiers::empty()),
		}
	}
}

#[derive(Debug, Clone)]
struct AnimationState {
	active: bool,
	start_time: Instant,
	duration: Duration,
	// Other animation state data
}

impl Default for AnimationState {
	fn default() -> Self {
		Self {
			active: false,
			start_time: Instant::now(),
			duration: Duration::from_millis(300)
		}
	}
}

#[repr(u8)]
enum ConnectionEvent {
	Initialize(Sender<ConnectionEvent>),
	Connect(ServerConnection),
	Disconnect,
	MessageReceived(Result<Ssp, Error>),
	SendMessage(Ssp)
}

#[repr(u8)]
enum ConnectionState {
	Uninitialized,
	Disconnected(Receiver<ConnectionEvent>),
	Connected(ServerConnection, Receiver<ConnectionEvent>)
}

#[derive(Debug, Clone)]
struct GameSettings {
	// Audio settings
	master_volume: f32,
	music_volume: f32,
	ui_volume: f32,
	// Keybinds
	keybinds: HashMap<(Key, Modifiers), Message>
}

impl Default for GameSettings {
	fn default() -> Self {
		let mut keybinds = HashMap::new();
		keybinds.insert(KeybindType::Pass.default_keybind(), Message::Bid(0));
		keybinds.insert(KeybindType::Bid2.default_keybind(), Message::Bid(2));
		keybinds.insert(KeybindType::Bid3.default_keybind(), Message::Bid(3));
		keybinds.insert(KeybindType::Bid4.default_keybind(), Message::Bid(4));
		keybinds.insert(KeybindType::Bid5.default_keybind(), Message::Bid(5));
		keybinds.insert(KeybindType::PlayCard1.default_keybind(), Message::PlayCardIndex(1));
		keybinds.insert(KeybindType::PlayCard2.default_keybind(), Message::PlayCardIndex(2));
		keybinds.insert(KeybindType::PlayCard3.default_keybind(), Message::PlayCardIndex(3));
		keybinds.insert(KeybindType::PlayCard4.default_keybind(), Message::PlayCardIndex(4));
		keybinds.insert(KeybindType::PlayCard5.default_keybind(), Message::PlayCardIndex(5));
		keybinds.insert(KeybindType::PlayCard6.default_keybind(), Message::PlayCardIndex(6));
		keybinds.insert(KeybindType::ToggleSettings.default_keybind(), Message::ToggleSettings);
		Self {
			master_volume: 100.0,
			music_volume: 100.0,
			ui_volume: 100.0,
			keybinds
		}
	}
}

impl GameSettings {
	///This will be used as a component for the settings scene or the settings popup in the game scene.
	pub fn view(&self) -> Element<Message> {
		// Audio Section
		let audio_header = text("Audio")
			.size(20)
			.style(TEXT_COLOR);

		let master_volume_slider = volume_slider(
			"Master Volume",
			self.master_volume,
			VolumeType::Master
		);

		let music_volume_slider = volume_slider(
			"Music Volume",
			self.music_volume,
			VolumeType::Music
		);

		let ui_volume_slider = volume_slider(
			"UI Volume",
			self.ui_volume,
			VolumeType::UI
		);

		let audio_section = column![
			audio_header,
			Space::with_height(Length::Units(10)),
			master_volume_slider,
			Space::with_height(Length::Units(10)),
			music_volume_slider,
			Space::with_height(Length::Units(10)),
			ui_volume_slider,
		]
		.spacing(5)
		.padding(10);

		// Keybinds Section
		let keybinds_header = text("Keybinds")
			.size(20)
			.style(TEXT_COLOR);

		// Bidding Keybinds
		let bidding_header = text("Bidding")
			.size(16)
			.style(TEXT_COLOR);

		let bidding_keybinds = column![
			bidding_header,
			keybind_row("Pass", KeybindType::Pass, keybind_for(KeybindType::Pass, keybinds)),
			keybind_row("Bid 2", KeybindType::Bid2, keybind_for(KeybindType::Bid2, keybinds)),
			keybind_row("Bid 3", KeybindType::Bid3, keybind_for(KeybindType::Bid3, keybinds)),
			keybind_row("Bid 4", KeybindType::Bid4, keybind_for(KeybindType::Bid4, keybinds)),
			keybind_row("Bid 5", KeybindType::Bid5, keybind_for(KeybindType::Bid5, keybinds)),
		]
		.spacing(5)
		.padding(10);

		// Card Playing Keybinds
		let playing_header = text("Playing Cards")
			.size(16)
			.style(TEXT_COLOR);

		let playing_keybinds = column![
			playing_header,
			keybind_row("Play Card 1", KeybindType::PlayCard1, keybind_for(KeybindType::PlayCard1, keybinds)),
			keybind_row("Play Card 2", KeybindType::PlayCard2, keybind_for(KeybindType::PlayCard2, keybinds)),
			keybind_row("Play Card 3", KeybindType::PlayCard3, keybind_for(KeybindType::PlayCard3, keybinds)),
			keybind_row("Play Card 4", KeybindType::PlayCard4, keybind_for(KeybindType::PlayCard4, keybinds)),
			keybind_row("Play Card 5", KeybindType::PlayCard5, keybind_for(KeybindType::PlayCard5, keybinds)),
			keybind_row("Play Card 6", KeybindType::PlayCard6, keybind_for(KeybindType::PlayCard6, keybinds)),
		]
		.spacing(5)
		.padding(10);

		// Miscellaneous Keybinds
		let misc_header = text("Miscellaneous")
			.size(16)
			.style(TEXT_COLOR);

		let misc_keybinds = column![
			misc_header,
			keybind_row("Toggle Settings", KeybindType::ToggleSettings, keybind_for(KeybindType::ToggleSettings, keybinds)),
		]
		.spacing(5)
		.padding(10);

		let keybinds_section = column![
			keybinds_header,
			Space::with_height(Length::Units(10)),
			bidding_keybinds,
			Space::with_height(Length::Units(10)),
			playing_keybinds,
			Space::with_height(Length::Units(10)),
			misc_keybinds,
		]
		.spacing(5)
		.padding(10);

		// Version Display
		let version = text(format!("Version: {}", env!("CARGO_PKG_VERSION")))
			.size(12)
			.style(TEXT_COLOR.with_alpha(0.7));

		// Full Layout
		let content = column![
			Space::with_height(Length::Units(20)),
			audio_section,
			Space::with_height(Length::Units(20)),
			keybinds_section,
			Space::with_height(Length::Units(20)),
			version,
		]
		.spacing(10)
		.padding(PADDING)
		.width(Length::Fill)
		.height(Length::Fill)
		.align_items(Alignment::Center);

		let scrollable_content = scrollable(content)
			.height(Length::Fill)
			.width(Length::Fill);

		container(scrollable_content)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
	}
}

/// Helper function to calculate player positions on the ellipse
fn calculate_player_position(index: usize, total: usize) -> (f32, f32) {
	let theta = 2.0 * PI * index as f32 / total as f32 + 3.0 * PI / 2.0;
	(-theta.cos() * ELLIPSE_RADIUS, -ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS)
}

// Implementation of methods for Setback
impl Setback {
	fn update(&mut self, message: Message) -> Command<Message> {
		match message {
			Message::TransitionTo(scene) => {
				self.scene = scene;
				Command::none()
			}
			Message::Back => {
				// Handle back navigation based on current scene
				match self.scene {
					Scene::Settings | Scene::Multiplayer | Scene::Setup => {
						self.scene = Scene::MainMenu;
					}
					Scene::Game => {
						// Show confirmation dialog before leaving game
						// In a real implementation, this would show a modal
					}
					_ => {}
				}
				Command::none()
			}
			Message::SingleplayerPressed => {
				self.scene = Scene::Setup;
				self.host = Host::SingleplayerSetup(GameBuilder::default());
				
				// Initialize setup with default players
				self.players = vec![
					(0, "You".to_string(), true, 0, (0.0, 0.0)), 
					(1, "Bot 1".to_string(), false, 1, (0.0, 0.0)),
					(2, "Bot 2".to_string(), false, 0, (0.0, 0.0)),
					(3, "Bot 3".to_string(), false, 1, (0.0, 0.0)),
				];
				
				// Update positions based on the ellipse formula
				for i in 0..self.players.len() {
					let (x, y) = calculate_player_position(i, self.players.len());
					self.players[i].4 = (x, y);
				}
				
				Command::none()
			}
			Message::MultiplayerPressed => {
				// Would handle connection flow here
				self.scene = Scene::Multiplayer;
				Command::none()
			}
			Message::SettingsPressed => {
				self.scene = Scene::Settings;
				Command::none()
			}
			Message::ExitPressed => {
				std::process::exit(0);
			}
			Message::TeamModeChanged(team_mode) => {
				self.team_mode = team_mode;
				Command::none()
			}
			Message::WinBy2Changed(win_by_2) => {
				self.win_by_2 = win_by_2;
				Command::none()
			}
			Message::NegativeScoresChanged(negative_scores) => {
				self.negative_scores = negative_scores;
				Command::none()
			}
			Message::DealerOverrideChanged(dealer_override) => {
				self.dealer_override = dealer_override;
				Command::none()
			}
			Message::MaxScoreChanged(max_score) => {
				self.max_score = max_score;
				Command::none()
			}
			Message::MaxPlayersChanged(max_players) => {
				self.max_players = max_players;
				Command::none()
			}
			Message::AddPlayer => {
				if self.players.len() < self.max_players as usize {
					let new_index = self.players.len();
					let team_id = (new_index % 2) as u8;
					
					self.players.push((
						new_index,
						format!("Bot {}", new_index),
						false,
						team_id,
						(0.0, 0.0),
					));
					
					// Recalculate all positions
					for i in 0..self.players.len() {
						let (x, y) = calculate_player_position(i, self.players.len());
						self.players[i].4 = (x, y);
					}
				}
				Command::none()
			}
			Message::RemovePlayer(index) => {
				if self.players.len() > 2 && index < self.players.len() {
					self.players.remove(index);
					
					// Update indices and recalculate positions
					for i in 0..self.players.len() {
						self.players[i].0 = i;
						let (x, y) = calculate_player_position(i, self.players.len());
						self.players[i].4 = (x, y);
					}
				}
				Command::none()
			}
			Message::UpdatePlayerTeam(index, team) => {
				if index < self.players.len() {
					self.players[index].3 = team;
				}
				Command::none()
			}
			Message::Start => {
				self.scene = Scene::SingleplayerGame(GameBuilder::default());
				Command::none()
			}
		}
	}

	fn subscription(&self) -> Subscription<Message> {
		let mut subscriptions: Vec<Subscription<Message>> = Vec::new();

		//animations
		if self.animation_state.active {
			subscriptions.push(iced::time::every(Duration::from_millis(16))
				.map(|instant| Message::AnimationTick(instant)));
		}

		//network connection
		subscriptions.push(
			Subscription::run(stream::channel(100, |mut output| async move {
				let mut connection_state: ConnectionState = ConnectionState::Uninitialized;
				loop {
					match connection_state {
						ConnectionState::Uninitialized => {
							//immediately initialize channel
							let (sender, receiver) = mpsc::channel(100);
							output.send(ConnectionEvent::Initialize(sender)).await.unwrap();
							connection_state = ConnectionState::Disconnected(receiver);
						}
						ConnectionState::Disconnected(receiver) => {
							if let ConnectionEvent::Connect(connection) = receiver.recv().await.unwrap() {
								connection_state = ConnectionState::Connected(connection, receiver);
							} else {
								unreachable!("Must connect before sending messages")
							}
						}
						ConnectionState::Connected(connection, receiver) => {
							//update connection state and send messages
							loop {
								match receiver.try_recv() {
									Ok(ConnectionEvent::Initialize(_)) => unreachable!("Already initialized"),
									Ok(ConnectionEvent::Connect) => unreachable!("Already connected"),
									Ok(ConnectionEvent::Disconnect) => {
										connection_state = ConnectionState::Disconnected(receiver);
									}
									Ok(ConnectionEvent::MessageReceived(_)) => unreachable!("Messages are received from connection"),
									Ok(ConnectionEvent::SendMessage(ssp)) => {
										if !connection.write(ssp).await {
											connection_state = ConnectionState::Disconnected(receiver);
										}
									}
									Err(TryRecvError::Empty) => break,
									Err(TryRecvError::Disconnected) => panic!()
								}
							}
							//receive messages from connection
							loop {
								match connection.ssp_rx.try_recv() {
									Ok(message) => output.send(ConnectionEvent::MessageReceived(message)).await.unwrap(),
									Err(TryRecvError::Empty) => break,
									Err(TryRecvError::Disconnected) => panic!()
								}
							}
						}
					}
				}
			}))
		);

		//singleplayer runtime game events
		if let Scene::SingleplayerGame(game) = &self.scene {
			subscriptions.push(
				Subscription::run(stream::channel(100, |mut output| async move {
					loop {
						output.send(game.event_emitter.recv().await.unwrap()).await.unwrap();
					}
				})).map(Message::GameEvent)
			);
		}

		//keypresses
		match self.scene {
			Scene::SingleplayerGame(_) | Scene::MultiplayerGame(_) => {
				subscriptions.push(on_key_press(|key, modifiers| {
					self.settings.keybinds.get(&(key, modifiers)).cloned()
				}));
			}
			_ => {}
		}

		Subscription::batch(subscriptions)
	}

	fn theme(&self) -> Theme {
		Theme::custom(
			String::new(),
			iced::theme::Palette {
				background: BACKGROUND_COLOR,
				text: TEXT_COLOR,
				primary: UI_ELEMENT_COLOR,
				success: SUCCESS_COLOR,
				danger: DANGER_COLOR,
			}
		)
	}

	// View implementations

	fn view(&self) -> Element<Message> {
		match &self.scene {
			Scene::MainMenu => views::main_menu(),
			Scene::Multiplayer(games, filter) => views::multiplayer(games.as_slice(), filter),
			Scene::Settings => views::settings(&self.settings),
			Scene::Setup(setup_data) => views::setup(setup_data, matches!(self.host, Host::Singleplayer | Host::Host)),
			Scene::SingleplayerGame(runtime) => self.view_game(runtime.game.read().unwrap()),
			Scene::MultiplayerGame(game) => self.view_game(game)
		}
	}

	fn view_game(&self, game: impl Ref<Game>) -> Element<Message> {
		if let Some(game_state) = &self.game_state {
			// Convert card representations for display
			let hand_strings: Vec<String> = game_state.hand.iter()
				.map(|card| format!("{:?} {:?}", card.suit, card.value))
				.collect();
				
			let center_pile_strings: Vec<String> = game_state.center_pile.iter()
				.map(|card| format!("{:?} {:?}", card.suit, card.value))
				.collect();
				
			let trump_suit_str = game_state.trump_suit.as_ref().map(|s| format!("{:?}", s));
			let lead_suit_str = game_state.lead_suit.as_ref().map(|s| format!("{:?}", s));
				
			views::game(
				game_state.round,
				&game_state.scores,
				trump_suit_str.as_deref(),
				lead_suit_str.as_deref(),
				game_state.highest_bid,
				&game_state.bidder_name,
				&game_state.pile_owner,
				game_state.active_player_index,
				self.local_player_index,
				&hand_strings,
				&center_pile_strings,
				&game_state.player_card_counts,
			)
		} else {
			// Fallback view if game state isn't initialized
			container(
				text("Game not initialized")
					.size(20)
					.style(TEXT_COLOR)
			)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
		}
	}
}