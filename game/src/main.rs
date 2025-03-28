#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate image as Img;

use {
	crate::components::*,
	bincode::{
		config::standard,
		de::{BorrowDecoder, Decoder},
		enc::Encoder,
		error::{DecodeError, EncodeError},
		BorrowDecode, Decode, Encode
	},
	env_logger::{Builder, Env},
	iced::{
		color,
		widget::{
			container::background,
			image, container, button, center, column, row, text
		},
		window,
		theme::Palette,
		alignment::{Horizontal, Vertical},
		advanced::image::Handle,
		Color, Element, Length, Subscription, Task, Theme
	},
	Img::ImageFormat,
	keyring::Entry,
	log::{debug, error, info, trace, warn},
	shared::{
		game::{GameBuilder, GameRuntime},
		Card, Comparison, Error, GameEvent, GameFilter, GameMeta, PlayerData, PlayerType, Protocol, PublicGame, RoundScores, ServerConnection, Ssp, Suit
	},
	std::{
		env,
		f32::consts::PI,
		fs::{self, File},
		path::{Path, PathBuf},
		sync::{Arc, LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard}
	},
	tokio::{
		runtime::Runtime,
		sync::mpsc::{unbounded_channel, UnboundedSender}
	}
};

mod components;

static PROJ_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	env::current_exe()
		.ok()
		.and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
		.unwrap()
	});
static USERNAME_PATH: LazyLock<PathBuf> = LazyLock::new(||{PROJ_DIR.join(".user")});
static SETTINGS_PATH: LazyLock<PathBuf> = LazyLock::new(||{PROJ_DIR.join(".config")});
// static ASSETS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
// 	#[cfg(debug_assertions)]
// 	{
// 		// During development, use relative path from project root
// 		PathBuf::from("game\\assets")
// 	}
// 	#[cfg(not(debug_assertions))]
// 	{
// 		PROJ_DIR.join("assets")
// 	}
// });

const LOG: bool = false;
const SERVICE: &str = "com.github.setback-game.setback";
const BACKGROUND_COLOR: Color = color!(0x114411);

//images

const BOT_ICON: &[u8] = include_bytes!("../assets/images/botIcon.png");
const CARDS_ICON: &[u8] = include_bytes!("../assets/images/cardsIcon.png");
const CHECKMARK: &[u8] = include_bytes!("../assets/images/checkmark.png");
const DROPDOWN_ARROW: &[u8] = include_bytes!("../assets/images/dropdown-arrow.png");
const GEAR: &[u8] = include_bytes!("../assets/images/gear.png");
const PERSON_ICON: &[u8] = include_bytes!("../assets/images/personIcon.png");
const VOLUME_HIGH: &[u8] = include_bytes!("../assets/images/volume-high.png");
const VOLUME_LOW: &[u8] = include_bytes!("../assets/images/volume-low.png");
const VOLUME_MUTE: &[u8] = include_bytes!("../assets/images/volume-mute.png");
const X_ICON: &[u8] = include_bytes!("../assets/images/x.png");

//suit images

const CLUBS_ICON: &[u8] = include_bytes!("../assets/images/suits/clubs.png");
const DIAMONDS_ICON: &[u8] = include_bytes!("../assets/images/suits/diamonds.png");
const HEARTS_ICON: &[u8] = include_bytes!("../assets/images/suits/hearts.png");
const SPADES_ICON: &[u8] = include_bytes!("../assets/images/suits/spades.png");

//card images

macro_rules! card_img {
	($const_name:ident, $number:expr, $suit:expr) => {
		const $const_name: &[u8] = include_bytes!(
			concat!("../assets/images/cards/", $number, "_of_", $suit, ".png")
		);
	};
}
card_img!(ACE_OF_SPADES, "ace", "spades");
card_img!(ACE_OF_DIAMONDS, "ace", "diamonds");
card_img!(ACE_OF_CLUBS, "ace", "clubs");
card_img!(ACE_OF_HEARTS, "ace", "hearts");
card_img!(TWO_OF_SPADES, 2, "spades");
card_img!(TWO_OF_DIAMONDS, 2, "diamonds");
card_img!(TWO_OF_CLUBS, 2, "clubs");
card_img!(TWO_OF_HEARTS, 2, "hearts");
card_img!(THREE_OF_SPADES, 3, "spades");
card_img!(THREE_OF_DIAMONDS, 3, "diamonds");
card_img!(THREE_OF_CLUBS, 3, "clubs");
card_img!(THREE_OF_HEARTS, 3, "hearts");
card_img!(FOUR_OF_SPADES, 4, "spades");
card_img!(FOUR_OF_DIAMONDS, 4, "diamonds");
card_img!(FOUR_OF_CLUBS, 4, "clubs");
card_img!(FOUR_OF_HEARTS, 4, "hearts");
card_img!(FIVE_OF_SPADES, 5, "spades");
card_img!(FIVE_OF_DIAMONDS, 5, "diamonds");
card_img!(FIVE_OF_CLUBS, 5, "clubs");
card_img!(FIVE_OF_HEARTS, 5, "hearts");
card_img!(SIX_OF_SPADES, 6, "spades");
card_img!(SIX_OF_DIAMONDS, 6, "diamonds");
card_img!(SIX_OF_CLUBS, 6, "clubs");
card_img!(SIX_OF_HEARTS, 6, "hearts");
card_img!(SEVEN_OF_SPADES, 7, "spades");
card_img!(SEVEN_OF_DIAMONDS, 7, "diamonds");
card_img!(SEVEN_OF_CLUBS, 7, "clubs");
card_img!(SEVEN_OF_HEARTS, 7, "hearts");
card_img!(EIGHT_OF_SPADES, 8, "spades");
card_img!(EIGHT_OF_DIAMONDS, 8, "diamonds");
card_img!(EIGHT_OF_CLUBS, 8, "clubs");
card_img!(EIGHT_OF_HEARTS, 8, "hearts");
card_img!(NINE_OF_SPADES, 9, "spades");
card_img!(NINE_OF_DIAMONDS, 9, "diamonds");
card_img!(NINE_OF_CLUBS, 9, "clubs");
card_img!(NINE_OF_HEARTS, 9, "hearts");
card_img!(TEN_OF_SPADES, 10, "spades");
card_img!(TEN_OF_DIAMONDS, 10, "diamonds");
card_img!(TEN_OF_CLUBS, 10, "clubs");
card_img!(TEN_OF_HEARTS, 10, "hearts");
card_img!(JACK_OF_SPADES, "jack", "spades");
card_img!(JACK_OF_DIAMONDS, "jack", "diamonds");
card_img!(JACK_OF_CLUBS, "jack", "clubs");
card_img!(JACK_OF_HEARTS, "jack", "hearts");
card_img!(QUEEN_OF_SPADES, "queen", "spades");
card_img!(QUEEN_OF_DIAMONDS, "queen", "diamonds");
card_img!(QUEEN_OF_CLUBS, "queen", "clubs");
card_img!(QUEEN_OF_HEARTS, "queen", "hearts");
card_img!(KING_OF_SPADES, "king", "spades");
card_img!(KING_OF_DIAMONDS, "king", "diamonds");
card_img!(KING_OF_CLUBS, "king", "clubs");
card_img!(KING_OF_HEARTS, "king", "hearts");

fn main() -> iced::Result {
	// //Tokio runtime
	// let runtime: Runtime = tokio::runtime::Builder::new_multi_thread()
	// 	.enable_all()
	// 	.build()
	// 	.unwrap();

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

	// //Game state
	// let (event_tx, mut event_rx) = unbounded_channel();
	// let game: Arc<RwLock<SetbackGame>> = Arc::new(RwLock::new(SetbackGame {
	// 	scene_type: Scene::MainMenu,
	// 	host_type: Host::None,
	// 	connection: None,
	// 	event_tx,
	// 	local_player_index: 0
	// }));

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
		.theme(Setback::dark_theme)
		.centered()
		.antialiasing(true)
		.run()
}

#[derive(Default)]
struct Setback {
	scene: Scene,
	host: Host,
	connection: Option<ServerConnection>,
	local_player_index: u8,
	settings: Settings
}

impl Setback {
	fn update(&mut self, event: Event) -> Task<Event> {
		match event {
			Event::ClickSingleplayer => {
				self.scene = Scene::Setup;
				self.host = Host::SingleplayerSetup(GameBuilder::default());
			}
			Event::ClickMultiplayer => {
				todo!()
			}
			Event::ClickSettings => {
				self.scene = Scene::Settings;
			}
			Event::ClickExit => {
				std::process::exit(0);
			}
		}
		Task::none()
	}

	fn subscription(&self) -> Subscription<Event> {
		Subscription::none()
	}

	fn view(&self) -> impl Into<Element<Event>> {
		// match self.scene {
		// 	Scene::MainMenu => {
		// 		todo!()
		// 	}
		// 	Scene::Multiplayer => {
		// 		todo!()
		// 	}
		// 	Scene::Settings => {
		// 		todo!()
		// 	}
		// 	Scene::Setup => {
		// 		todo!()
		// 	}
		// 	Scene::Game => {
		// 		todo!()
		// 	}
		// }
		container(
			image(Handle::from_bytes(CARDS_ICON))
		)
	}

	fn dark_theme(&self) -> Theme {
		Theme::custom(String::new(), Palette { //why do I need to pass a string here?
			background: color!(0x114411),
			text: color!(0xCCFFCC),
			primary: color!(0xffffff),
			success: color!(0xffffff),
			danger: color!(0xCC0000)
		})
	}

	// fn light_theme(&self) -> Theme {
	// 	Theme::custom(String::new(), Palette {
	// 		background: color!(0x477c47),
	// 		text: Color::BLACK,
	// 		primary: color!(0xFFFFFF),
	// 		success: color!(0x324731),
	// 		danger: color!(0xCC0000),
	// 	})
	// }
}

#[repr(u8)]
#[derive(Debug, Clone)] //why do I need to derive Clone here?
enum Event {
	ClickSingleplayer,
	ClickMultiplayer,
	ClickSettings,
	ClickExit,
}

///Game state which stores what scene is currently active.
#[repr(u8)]
#[derive(Default)]
enum Scene {
	#[default]
	MainMenu,
	Multiplayer,
	Settings,
	Setup,
	Game
}

#[repr(u8)]
#[derive(Default)]
enum Host {
	#[default]
	None,
	SingleplayerSetup(GameBuilder),
	SingleplayerGame(GameRuntime),
	Host,
	Client
}

// #[repr(u8)]
// enum UiEvent {
// 	PromptCredentials,
// 	ConnectionSuccess,
// 	AuthenticationError,
// 	UpdateTeam{index: u8, team: i32},
// 	UpdatePlayerType{index: u8, human: bool},
// 	// UpdatePlayers(Vec<SetupPlayerData>),
// 	UpdatePublicGames(Vec<PublicGame>),
// 	UpdateGame(GameMeta),
// 	UnrecoverableError(String),
// 	// RecoverableError(String, ErrorLocation),
// 	PromptBid,
// 	PromptCard,
// 	RepromptBid,
// 	RepromptCard,
// 	PlayerBid{index: u8, bid: u8},
// 	PlayerPlayedCard{index: u8, card: Card},
// 	PlayerReadyUp{index: u8, ready: bool},
// 	PlayerTurn{index: u8},
// 	RoundScores(RoundScores),
// 	GameOver(Vec<i8>),
// 	DealCards,
// 	UpdateHand(Vec<Card>),
// 	StartGame,
// 	EnterSetup{host: bool}
// }

// impl From<Comparison> for Comparison {
// 	fn from(comparison: Comparison) -> Self {
// 		match comparison {
// 			Comparison::Equal => Comparison::Equal,
// 			Comparison::LessOrEqual => Comparison::LessOrEqual,
// 			Comparison::GreaterOrEqual => Comparison::GreaterOrEqual,
// 			Comparison::Less => Comparison::Less,
// 			Comparison::Greater => Comparison::Greater,
// 			Comparison::NotEqual => Comparison::NotEqual
// 		}
// 	}
// }

// impl From<Suit> for shared::Suit {
// 	fn from(suit: Suit) -> Self {
// 		match suit {
// 			Suit::Hearts => shared::Suit::Hearts,
// 			Suit::Clubs => shared::Suit::Clubs,
// 			Suit::Diamonds => shared::Suit::Diamonds,
// 			Suit::Spades => shared::Suit::Spades
// 		}
// 	}
// }

// impl From<Card> for Card {
// 	fn from(card: Card) -> Self {
// 		Card {suit: card.suit.into(), number: card.number as u8}
// 	}
// }

#[derive(Encode, Decode)]
struct Settings {
	master_volume: f32,
	music_volume: f32,
	ui_volume: f32,
	pass_keybind: String,
	bid_2_keybind: String,
	bid_3_keybind: String,
	bid_4_keybind: String,
	bid_5_keybind: String,
	play_card_1_keybind: String,
	play_card_2_keybind: String,
	play_card_3_keybind: String,
	play_card_4_keybind: String,
	play_card_5_keybind: String,
	play_card_6_keybind: String,
	toggle_settings_keybind: String
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			master_volume: 100.0,
			music_volume: 100.0,
			ui_volume: 100.0,
			pass_keybind: String::from("0"),
			bid_2_keybind: String::from("2"),
			bid_3_keybind: String::from("3"),
			bid_4_keybind: String::from("4"),
			bid_5_keybind: String::from("5"),
			play_card_1_keybind: String::from("1"),
			play_card_2_keybind: String::from("2"),
			play_card_3_keybind: String::from("3"),
			play_card_4_keybind: String::from("4"),
			play_card_5_keybind: String::from("5"),
			play_card_6_keybind: String::from("6"),
			toggle_settings_keybind: String::from("esc")
		}
	}
}

// //Main menu functions

// ///Prompts user for credentials if necessary or creates connection to server with saved credentials.
// fn multiplayer_clicked(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
// 	{
// 		let game: RwLockReadGuard<SetbackGame> = setback_game.read().unwrap();
// 		if game.connection.is_some() {
// 			game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
// 			info!("Already connected to server.");
// 			return;
// 		}
// 	}
// 	info!("Looking for saved username.");
// 	let path: &Path = USERNAME_PATH.as_path();
// 	match fs::read_to_string(path) {
// 		Ok(username) => {
// 			info!("Found saved username, looking for password from secure storage.");
// 			match Entry::new(SERVICE, username.as_str()).unwrap().get_password() {
// 				Ok(password) => {
// 					info!("Found password in secure storage, connecting to server.");
// 					runtime.spawn(async move {
// 						match ServerConnection::new_connection(
// 							username,
// 							password,
// 							ssp_callback(setback_game.clone()),
// 							handle_error(setback_game.clone())
// 						).await {
// 							Ok(connection) => {
// 								info!("Connected to server.");
// 								let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 								game.connection = Some(connection);
// 								game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
// 							}
// 							Err(error) => {
// 								if let Error::Authentication = error {
// 									info!("Authentication error, prompting user for credentials.");
// 									setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
// 								} else {
// 									error!("Unexpected error connecting to server.");
// 									setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::MultiplayerPopup)).unwrap();
// 								}
// 							}
// 						}
// 					});
// 				}
// 				Err(keyring::Error::NoEntry) => {
// 					info!("No password found, prompting user for credentials.");
// 					File::create(path).unwrap();
// 					setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
// 				}
// 				Err(keyring::Error::Ambiguous(_)) => {
// 					error!("Ambiguous credentials. User may need to manually remove conflicting password from secure storage. Prompting user for credentials.");
// 					setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
// 				}
// 				Err(_) => {unreachable!()}
// 			}
// 		}
// 		Err(_) => {
// 			info!("No saved username found, prompting user for credentials.");
// 			File::create(path).unwrap();
// 			setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
// 		}
// 	}
// }

// ///When the user submits their credentials, this function is called.
// fn handle_credentials(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, username: SharedString, password: SharedString) {
// 	runtime.spawn(async move {
// 		info!("Attempting to connect to server with provided credentials.");
// 		let username: String = username.to_string();
// 		let password: String = password.to_string();
// 		match ServerConnection::new_connection(
// 			username.clone(),
// 			password.clone(),
// 			ssp_callback(setback_game.clone()),
// 			handle_error(setback_game.clone())
// 		).await {
// 			Ok(connection) => {
// 				info!("Connected to server.");
// 				let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 				game.connection = Some(connection);
// 				game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
// 				fs::write(USERNAME_PATH.as_path(), username.as_str()).unwrap();
// 				if let Err(keyring::Error::Ambiguous(_)) = Entry::new(SERVICE, username.as_str()).unwrap().set_password(password.as_str()) {
// 					error!("Ambiguous credentials. User may need to manually remove conflicting password from secure storage.");
// 				}
// 			}
// 			Err(error) => {
// 				if let Error::Authentication = error {
// 					info!("Authentication error, reprompting user for credentials.");
// 					setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
// 				} else {
// 					error!("Unexpected error connecting to server.");
// 					setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::MultiplayerPopup)).unwrap();
// 				}
// 			}
// 		}
// 	});
// }

// ///Saves settings to disk
// fn save_settings(settings: Settings) {
// 	bincode::encode_into_std_write(settings, &mut File::create(SETTINGS_PATH.as_path()).unwrap(), standard()).unwrap();
// }

// ///Loads settings from disk
// fn load_settings() -> Settings {
// 	match File::open(SETTINGS_PATH.as_path()) {
// 		Ok(mut file) => {
// 			bincode::decode_from_std_read(&mut file, standard()).unwrap()
// 		}
// 		Err(_) => {
// 			let settings: Settings = Settings::new();
// 			bincode::encode_into_std_write(&settings, &mut File::create(SETTINGS_PATH.as_path()).unwrap(), standard()).unwrap();
// 			settings
// 		}
// 	}
// }

// //singleplayer and multiplayer functions

// ///Sets the game state to the given scene and host.
// fn set_state(setback_game: Arc<RwLock<SetbackGame>>, scene: SceneType, host: HostType) {
// 	info!("Setting scene to {:?} and host to {:?}.", scene, host);
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	match game.scene_type {
// 		Scene::MainMenu => {
// 			match scene {
// 				SceneType::Game => {unreachable!("You should not be able to change the scene to a game when you're in the main menu.")}
// 				SceneType::Setup => {
// 					// game.event_tx.send(UiEvent::UpdatePlayers(ModelRc::from(Vec::new().as_slice()))).unwrap();
// 				}
// 				_ => {game.scene_type = scene.into();}
// 			}
// 		}
// 		Scene::Setup => {
// 			match scene {
// 				SceneType::Multiplayer |
// 				SceneType::Settings => {unreachable!("You should not be able to change the scene to a multiplayer or settings when you're in the setup.")}
// 				_ => {game.scene_type = scene.into();}
// 			}
// 		}
// 		Scene::Multiplayer => {
// 			match scene {
// 				SceneType::Game |
// 				SceneType::Settings => {unreachable!("You should not be able to change the scene to a game or settings when you're in the multiplayer.")}
// 				SceneType::Setup => {
// 					// game.event_tx.send(UiEvent::UpdatePlayers(ModelRc::from(Vec::new().as_slice()))).unwrap();
// 				}
// 				_ => {game.scene_type = scene.into();}
// 			}
// 		}
// 		Scene::Game => {
// 			match scene {
// 				SceneType::Setup |
// 				SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the game.")}
// 				_ => {game.scene_type = scene.into();}
// 			}
// 		}
// 		Scene::Settings => {
// 			match scene {
// 				SceneType::Setup |
// 				SceneType::Multiplayer | SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the settings.")}
// 				_ => {game.scene_type = scene.into();}
// 			}
// 		}
// 	}
// 	match &game.host_type {
// 		Host::None => {
// 			match host {
// 				HostType::None | HostType::Host | HostType::Client => {}
// 				HostType::SingleplayerSetup => {
// 					//starting singleplayer game
// 					game.connection = None;
// 					game.host_type = Host::SingleplayerSetup(GameBuilder::default());
// 				}
// 				_ => {unreachable!("Illegal change in host type.")}
// 			}
// 		}
// 		Host::SingleplayerSetup(_) => {
// 			match host {
// 				HostType::None => {game.host_type = Host::None;}//leaving the singleplayer game setup
// 				HostType::SingleplayerSetup => {}//no change in host
// 				HostType::SingleplayerPlaying => {
// 					//starting singleplayer game
// 					let Host::SingleplayerSetup(game_builder) = std::mem::replace(&mut game.host_type, Host::None) else {unreachable!()};
// 					let game_runtime: GameRuntime = game_builder.start_game();
// 					game.host_type = Host::SingleplayerPlaying(game_runtime);
// 				}
// 				_ => {unreachable!("Illegal change in host type.")}
// 			}
// 		}
// 		Host::SingleplayerPlaying(game_runtime) => {
// 			match host {
// 				HostType::None => {
// 					//leaving the singleplayer game
// 					game_runtime.cancel_token.cancel();
// 					game.host_type = Host::None;
// 				}
// 				HostType::SingleplayerPlaying => {}//no change in host
// 				_ => {unreachable!("Illegal change in host type.")}
// 			}
// 		}
// 		Host::Host => {
// 			match host {
// 				HostType::None => {
// 					//leaving the multiplayer game
// 					game.connection = None;
// 					game.host_type = Host::None;
// 				}
// 				HostType::Host => {}//no change in host
// 				_ => {unreachable!("Illegal change in host type.")}
// 			}
// 		}
// 		Host::Client => {
// 			match host {
// 				HostType::None => {
// 					//leaving the multiplayer game
// 					game.connection = None;
// 					game.host_type = Host::None;
// 				}
// 				HostType::Client => {}//no change in host
// 				_ => {unreachable!("Illegal change in host type.")}
// 			}
// 		}
// 	}
// }

// ///Generic error handler.
// fn handle_error(setback_game: Arc<RwLock<SetbackGame>>) -> impl Fn(Error) + Send + 'static {
// 	move |error: Error| {
// 		warn!("Received error from server: {:?}.", error);
// 		match error {
// 			Error::Indeterminate |
// 			Error::Authentication |
// 			Error::Authorization |
// 			Error::NetworkIO |
// 			Error::ServerError |
// 			Error::IncompatibleVersion |
// 			Error::InvalidProtocol |
// 			Error::GamesAtCapacity => {
// 				setback_game.write().unwrap().event_tx.send(UiEvent::UnrecoverableError(error.to_string())).unwrap();
// 			}
// 			Error::GameAtCapacity |
// 			Error::GameNotFound => {
// 				match setback_game.read().unwrap().scene_type {
// 					Scene::MainMenu => {setback_game.write().unwrap().event_tx.send(UiEvent::UnrecoverableError(error.to_string())).unwrap();}
// 					Scene::Multiplayer => {setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap();}
// 					_ => {unreachable!("Error in illegal scene.")}
// 				}
// 			}
// 			Error::InvalidBid |
// 			Error::InvalidCard |
// 			Error::OutOfTurn => {
// 				setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap();
// 			}
// 			Error::ClientNotFound => unreachable!("Server-only error"),
// 		}
// 	}
// }

// fn add_player(setback_game: Arc<RwLock<SetbackGame>>) {
// 	info!("Adding player.");
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	let local_player_index: u8 = game.local_player_index;
// 	match &mut game.host_type {
// 		Host::SingleplayerSetup(game_builder) => {
// 			game_builder.add_ai();
// 			let players: Vec<SetupPlayerData> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
// 			game.event_tx.send(UiEvent::UpdatePlayers(players)).unwrap();
// 		}
// 		Host::Host => {
// 			game.connection.as_mut().unwrap().send_add_player();
// 		}
// 		_ => unreachable!("Should not be able to add player")
// 	}
// }

// fn remove_player(setback_game: Arc<RwLock<SetbackGame>>, index: u8) {
// 	info!("Removing player at index {}.", index);
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	let local_player_index: u8 = game.local_player_index;
// 	match &mut game.host_type {
// 		Host::SingleplayerSetup(game_builder) => {
// 			game_builder.game.players.remove(index as usize);
// 			let players: Vec<SetupPlayerData> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
// 			game.event_tx.send(UiEvent::UpdatePlayers(players)).unwrap();
// 		}
// 		Host::Host => {
// 			//This will kick the human player from the game if one is connected
// 			game.connection.as_mut().unwrap().send_remove_player(index);
// 		}
// 		_ => unreachable!("Should not be able to remove player")
// 	}
// }

// fn set_player_team(setback_game: Arc<RwLock<SetbackGame>>, index: u8, team: i32) {
// 	info!("Setting player team at index {} to team {}.", index, team);
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	match &mut game.host_type {
// 		Host::SingleplayerSetup(game_builder) => {
// 			game_builder.update_team(index, team as u8);
// 			game.event_tx.send(UiEvent::UpdateTeam{index, team}).unwrap();
// 		}
// 		Host::Host | Host::Client => {
// 			game.connection.as_mut().unwrap().send_update_team(index, team as u8);
// 		}
// 		_ => unreachable!("Should not be able to set player team")
// 	}
// }

// fn make_bid(setback_game: Arc<RwLock<SetbackGame>>, bid: i32) {
// 	info!("Making bid of {}.", bid);
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	let local_player_index: u8 = game.local_player_index;
// 	match &mut game.host_type {
// 		Host::SingleplayerPlaying(game_runtime) => {
// 			game_runtime.prompt_senders[local_player_index as usize].0.try_send(bid as u8).unwrap();
// 			game.event_tx.send(UiEvent::PlayerBid{index: local_player_index, bid: bid as u8}).unwrap();
// 		}
// 		Host::Host | Host::Client => {
// 			game.connection.as_mut().unwrap().send_game_event(GameEvent::PlayerBid{index: local_player_index, amount: bid as u8});
// 		}
// 		_ => unreachable!("Should not be able to make bid")
// 	}
// }

// fn play_card(setback_game: Arc<RwLock<SetbackGame>>, card: Card) {
// 	info!("Playing card: {:?}.", card);
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	let local_player_index: u8 = game.local_player_index;
// 	let card: Card = card.into();
// 	match &mut game.host_type {
// 		Host::SingleplayerPlaying(game_runtime) => {
// 			game_runtime.prompt_senders[local_player_index as usize].1.try_send(card).unwrap();
// 			game.event_tx.send(UiEvent::PlayerPlayedCard{index: local_player_index, card}).unwrap();
// 		}
// 		Host::Host | Host::Client => {
// 			game.connection.as_mut().unwrap().send_game_event(GameEvent::PlayerPlayedCard{index: 0, card});
// 		}
// 		_ => unreachable!("Should not be able to play card")
// 	}
// }

// //multiplayer only functions

// fn create_game(setback_game: Arc<RwLock<SetbackGame>>, public: bool) {
// 	info!("Creating game.");
// 	setback_game.write().unwrap().connection.as_mut().unwrap().send_create_game(public);
// }

// fn join_game(setback_game: Arc<RwLock<SetbackGame>>, code: SharedString) {
// 	info!("Joining game with code: {}.", code);
// 	setback_game.write().unwrap().connection.as_mut().unwrap().send_join_game(code.to_string().parse::<u32>().unwrap());
// }

// fn search_games(
// 	setback_game: Arc<RwLock<SetbackGame>>,
// 	max_score_cmp: Comparison,
// 	max_score_value: i32,
// 	max_players_cmp: Comparison,
// 	max_players_value: i32,
// 	std_team_mode: bool,
// 	win_by_2: bool,
// 	negative_scores: bool,
// 	dealer_override: bool
// ) {
// 	info!("Searching for public games.");
// 	setback_game.write().unwrap().connection.as_mut().unwrap().send_search_games(GameFilter {
// 		std_team_mode,
// 		win_by_2,
// 		negative_scores,
// 		dealer_override,
// 		max_score_cmp: Comparison::from(max_score_cmp),
// 		max_score_value: max_score_value as i8,
// 		max_players_cmp: Comparison::from(max_players_cmp),
// 		max_players_value: max_players_value as u8
// 	});
// }

// fn send_game_meta(
// 	setback_game: Arc<RwLock<SetbackGame>>,
// 	std_team_mode: bool,
// 	win_by_2: bool,
// 	negative_scores: bool,
// 	dealer_override: bool,
// 	max_score: i8,
// 	max_players: u8
// ) {
// 	info!("Sending game meta.");
// 	setback_game.write().unwrap().connection.as_mut().unwrap().send_game_meta(GameMeta {
// 		std_team_mode,
// 		win_by_2,
// 		negative_scores,
// 		dealer_override,
// 		max_score,
// 		max_players
// 	});
// }

// fn ready_up(setback_game: Arc<RwLock<SetbackGame>>, ready: bool) {
// 	info!("Readying up.");
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();	
// 	let local_player_index: u8 = game.local_player_index;
// 	game.connection.as_mut().unwrap().send_ready(ready, local_player_index);
// }

// fn logout(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
// 	info!("Logging out.");
// 	setback_game.write().unwrap().connection.as_mut().unwrap().logout(runtime);
// }

// fn set_player_type(setback_game: Arc<RwLock<SetbackGame>>, index: u8, human: bool) {
// 	info!("Setting player type at index {} to {}.", index, if human { "human" } else { "AI" });
// 	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
// 	assert!(matches!(game.host_type, Host::Host), "Only the host can set player types.");
// 	game.connection.as_mut().unwrap().send_update_player_type(index, human);
// }

// ///Ssp message received callback
// fn ssp_callback(game: Arc<RwLock<SetbackGame>>) -> impl Fn(Option<Ssp>) + Send + Sync + 'static {
// 	move |message: Option<Ssp>| {
// 		let mut game: RwLockWriteGuard<SetbackGame> = game.write().unwrap();
// 		match message {
// 			Some(ssp) => {
// 				info!("Received message from server: {:?}.", ssp);
// 				match ssp.protocol {
// 					Protocol::SyncID => {}
// 					Protocol::Err(error) => {
// 						match error {
// 							Error::InvalidBid => game.event_tx.send(UiEvent::RepromptBid).unwrap(),
// 							Error::InvalidCard => game.event_tx.send(UiEvent::RepromptCard).unwrap(),
// 							_ => game.event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap()
// 						}
// 					}
// 					Protocol::Game(game_meta, player_datas, host) => {
// 						game.event_tx.send(UiEvent::EnterSetup{host}).unwrap();
// 						game.event_tx.send(UiEvent::UpdateGame(game_meta)).unwrap();
// 						game.event_tx.send(UiEvent::UpdatePlayers(as_ui_player_vec(game.local_player_index, player_datas.as_slice()))).unwrap();
// 					}
// 					Protocol::PublicGames(public_games) => game.event_tx.send(UiEvent::UpdatePublicGames(public_games)).unwrap(),
// 					Protocol::GameMeta(game_meta) => game.event_tx.send(UiEvent::UpdateGame(game_meta)).unwrap(),
// 					Protocol::UpdatePlayerTeam{index, team} => game.event_tx.send(UiEvent::UpdateTeam{index, team: team as i32}).unwrap(),
// 					Protocol::UpdatePlayerType{index, human} => game.event_tx.send(UiEvent::UpdatePlayerType{index, human}).unwrap(),
// 					Protocol::Players{players, local_player_index} => game.event_tx.send(UiEvent::UpdatePlayers(as_ui_player_vec(local_player_index, players.as_slice()))).unwrap(),
// 					Protocol::Ready{is_ready, index} => game.event_tx.send(UiEvent::PlayerReadyUp{index, ready: is_ready}).unwrap(),
// 					Protocol::Start => game.event_tx.send(UiEvent::StartGame).unwrap(),
// 					Protocol::GameEvent(game_event) => {
// 						match game_event {
// 							GameEvent::PlayerBid{index, amount} => game.event_tx.send(UiEvent::PlayerBid{index, bid: amount}).unwrap(),
// 							GameEvent::PlayerPlayedCard{index, card} => game.event_tx.send(UiEvent::PlayerPlayedCard{index, card}).unwrap(),
// 							GameEvent::PlayerTurn{index} => game.event_tx.send(UiEvent::PlayerTurn{index}).unwrap(),
// 							GameEvent::PromptBid{hand,..} => {
// 								game.event_tx.send(UiEvent::UpdateHand(hand)).unwrap();
// 								game.event_tx.send(UiEvent::PromptBid).unwrap();
// 							}
// 							GameEvent::PromptCard{hand,..} => {
// 								game.event_tx.send(UiEvent::UpdateHand(hand)).unwrap();
// 								game.event_tx.send(UiEvent::PromptCard).unwrap();
// 							}
// 							GameEvent::DealCards => game.event_tx.send(UiEvent::DealCards).unwrap(),
// 							GameEvent::RoundScores(round_scores) => game.event_tx.send(UiEvent::RoundScores(round_scores)).unwrap(),
// 							GameEvent::GameOver(team_scores) => game.event_tx.send(UiEvent::GameOver(team_scores)).unwrap(),
// 							GameEvent::GameError(error) => game.event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap(),
// 						}
// 					}
// 					Protocol::Disconnect => unreachable!("Server doesn't send this to clients"),
// 					Protocol::Admin(_) => unreachable!("Clients are not permitted to send or receive admin messages."),
// 					_ => unreachable!("Request only")
// 				}
// 			}
// 			None => {
// 				info!("Connection lost.");
// 				game.connection = None;
// 				game.event_tx.send(UiEvent::UnrecoverableError(String::from("Connection lost."))).unwrap();
// 			}
// 		}
// 	}
// }

// //Util functions

// const ELLIPSE_RADIUS: f32 = 525.;
// const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

// fn as_ui_player_vec(local_player_index: u8, players: &[impl AsRef<PlayerData>]) -> Vec<SetupPlayerData> {
// 	let mut players: Vec<SetupPlayerData> = players
// 		.iter()
// 		.enumerate()
// 		.map(|(index, player)| {
// 			let theta: f32 = 2. * PI * index as f32 / players.len() as f32 + 3. * PI / 2.;
// 			SetupPlayerData {
// 				local: false,
// 				is_human: !matches!(player.as_ref().player_type, PlayerType::AI),
// 				team: player.as_ref().team_id as i32,
// 				username: SharedString::from(&player.as_ref().username),
// 				ready: match player.as_ref().player_type {
// 					PlayerType::MultiplayerHuman{ready,..} => ready,
// 					_ => true
// 				},
// 				x: -theta.cos() * ELLIPSE_RADIUS,
// 				y: -ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS
// 			}
// 		})
// 		.collect::<Vec<SetupPlayerData>>();
// 	players[local_player_index as usize].local = true;
// 	players
// }