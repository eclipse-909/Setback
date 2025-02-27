#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	bincode::{
		config::standard, de::{BorrowDecoder, Decoder}, enc::Encoder, error::{DecodeError, EncodeError}, BorrowDecode, Decode, Encode
	},
	env_logger::{Builder, Env},
	keyring::Entry,
	shared::{
		game::{GameBuilder, GameRuntime},
		Error, GameEvent, GameFilter, GameMeta, PlayerData, PlayerType, Protocol, PublicGame, RoundScores, ServerConnection, Ssp
	},
	slint::{Model, ModelRc, SharedString, Weak, VecModel},
	std::{
		any::Any, env, f32::consts::PI, fs::{self, File}, path::{Path, PathBuf}, sync::{Arc, LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard}
	},
	tokio::{
		runtime::{Handle, Runtime},
		sync::mpsc::{unbounded_channel, UnboundedSender}
	}
};

slint::include_modules!();

static PROJ_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	env::current_exe()
		.ok()
		.and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
		.unwrap()
});

static USERNAME_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join(".user")
});

static SETTINGS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join(".config")
});

static LOG_TX: std::sync::OnceLock<UnboundedSender<Log>> = std::sync::OnceLock::new();

const SERVICE: &str = "com.github.setback-game.setback";

macro_rules! info {
	($($args:expr),*) => {
		LOG_TX.get().unwrap().send(Log::Info(format!($($args),*))).unwrap();
	};
}
macro_rules! warn {
	($($args:expr),*) => {
		LOG_TX.get().unwrap().send(Log::Warn(format!($($args),*))).unwrap();
	};
}
macro_rules! error {
	($($args:expr),*) => {
		LOG_TX.get().unwrap().send(Log::Error(format!($($args),*))).unwrap();
	};
}
macro_rules! debug {
	($($args:expr),*) => {
		LOG_TX.get().unwrap().send(Log::Debug(format!($($args),*))).unwrap();
	};
}
macro_rules! trace {
	($($args:expr),*) => {
		LOG_TX.get().unwrap().send(Log::Trace(format!($($args),*))).unwrap();
	};
}

fn main() -> Result<(), slint::PlatformError> {
	//Tokio runtime
	let runtime: Runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

	//logger
	let (log_tx, mut log_rx) = unbounded_channel();
	LOG_TX.set(log_tx).unwrap();
	runtime.spawn(async move {
		//logger
		#[cfg(debug_assertions)]
		{
			// In debug builds, use default behavior (stderr)
			Builder::from_env(Env::default().default_filter_or("trace"))
				.init();
		}
		#[cfg(not(debug_assertions))]
		{
			// In release builds, log to file
			Builder::from_env(Env::default().default_filter_or("trace"))
				.target(env_logger::Target::Pipe(Box::new(File::create(PROJ_DIR.join("log.txt")).unwrap())))
				.init();
		}
		while let Some(log) = log_rx.recv().await {
			match log {
				Log::Info(message) => log::info!("{}", message),
				Log::Warn(message) => log::warn!("{}", message),
				Log::Error(message) => log::error!("{}", message),
				Log::Debug(message) => log::debug!("{}", message),
				Log::Trace(message) => log::trace!("{}", message)
			}
		}
	});

	//Game state
	let (event_tx, mut event_rx) = unbounded_channel();
	let game: Arc<RwLock<SetbackGame>> = Arc::new(RwLock::new(SetbackGame {
		scene_type: Scene::MainMenu,
		host_type: Host::None,
		connection: None,
		event_tx,
		local_player_index: 0
	}));
	
	//Create window
	let main_window: MainWindow = MainWindow::new()?;

	//UI event loop
	let window_weak: Weak<MainWindow> = main_window.as_weak();
	runtime.spawn(async move {
		while let Some(event) = event_rx.recv().await {
			window_weak.upgrade_in_event_loop(move |window| {
				match event {
					UiEvent::PromptCredentials => {
						window.set_multiplayer_popup_state(MultiplayerPopupState::PromptCredentials);
					}
					UiEvent::ConnectionSuccess => {
						window.set_multiplayer_popup_state(MultiplayerPopupState::Options);
						window.set_login_message(SharedString::from("Logging in..."));
					}
					UiEvent::AuthenticationError => {
						window.set_login_message(SharedString::from("Invalid username or password."));
					}
					UiEvent::UpdateTeam{index, team} => {
						let players: ModelRc<SetupPlayerData> = window.get_players();
						let mut player: SetupPlayerData = players.row_data(index as usize).unwrap();
						player.team = team;
						players.set_row_data(index as usize, player);
					}
					UiEvent::UpdatePlayerType{index, human} => {
						let players: ModelRc<SetupPlayerData> = window.get_players();
						let mut player: SetupPlayerData = players.row_data(index as usize).unwrap();
						player.is_human = human;
						players.set_row_data(index as usize, player);
					}
					UiEvent::UpdatePlayers(players) => {
						window.set_players(ModelRc::from(players.as_slice()));
					}
					// UiEvent::UpdatePlayer{index, num_players} => {
					// 	let players_model: ModelRc<SetupPlayerData> = window.get_players();
					// 	let players: &VecModel<SetupPlayerData> = players_model
					// 		.as_any()
					// 		.downcast_ref::<VecModel<SetupPlayerData>>()
					// 		.unwrap();
					// 	let mut player = players.remove(index as usize);
					// 	(player.x, player.y) = ellipse_position(index, num_players);
					// 	players.insert(index as usize, player);
					// }
					UiEvent::UpdatePublicGames(public_games) => {
						let games: Vec<Game> = public_games
							.iter()
							.map(|pub_game| Game {
								join_code: SharedString::from(pub_game.join_code.to_string()),
								players: pub_game.players as i32,
								max_players: pub_game.game_meta.max_players as i32,
								std_team: pub_game.game_meta.std_team_mode,
								win_by_2: pub_game.game_meta.win_by_2,
								negative_scores: pub_game.game_meta.negative_scores,
								dealer_override: pub_game.game_meta.dealer_override
							})
							.collect();
						window.set_games(ModelRc::from(games.as_slice()));
					}
					UiEvent::UpdateGame(game_meta) => {
						window.set_std_team_mode(game_meta.std_team_mode);
						window.set_win_by_2(game_meta.win_by_2);
						window.set_negative_scores(game_meta.negative_scores);
						window.set_dealer_override(game_meta.dealer_override);
						window.set_max_score(game_meta.max_score as i32);
						window.set_max_players(game_meta.max_players as i32);
					}
					UiEvent::UnrecoverableError(error) => {
						window.set_error_location(ErrorLocation::MainMenu);
						window.set_error_message(SharedString::from(&error));
						window.invoke_set_state(SceneType::MainMenu, HostType::None);
					}
					UiEvent::RecoverableError(error, location) => {
						window.set_error_location(location);
						window.set_error_message(SharedString::from(&error));
					}
					UiEvent::PromptBid => {}
					UiEvent::PromptCard => {}
					UiEvent::RepromptBid => {}
					UiEvent::RepromptCard => {}
					UiEvent::PlayerBid {index, bid} => {}
					UiEvent::PlayerPlayedCard {index, card} => {}
					UiEvent::PlayerReadyUp{index, ready} => {
						let players: ModelRc<SetupPlayerData> = window.get_players();
						let mut player: SetupPlayerData = players.row_data(index as usize).unwrap();
						player.ready = ready;
						players.set_row_data(index as usize, player);
					}
					UiEvent::PlayerTurn{index} => {}
					UiEvent::RoundScores(round_scores) => {}
					UiEvent::GameOver(items) => {}
					UiEvent::DealCards => {}
					UiEvent::UpdateHand(hand) => {}
					UiEvent::StartGame => {
						window.set_scene(SceneType::Game);
					}
					UiEvent::EnterSetup{host} => {
						window.set_scene(SceneType::Setup);
						window.set_host(if host {HostType::Host} else {HostType::Client});
					}
				}
			}).unwrap();
		}
	});

	//UI event handlers
	main_window.window().set_maximized(true);
	
	main_window.on_exit(||std::process::exit(0));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	let runtime_clone: Handle = runtime.handle().clone();
	main_window.on_multiplayer_clicked(move||multiplayer_clicked(game_clone.clone(), runtime_clone.clone()));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	let runtime_clone: Handle = runtime.handle().clone();
	main_window.on_handle_credentials(move|username, password|handle_credentials(game_clone.clone(), runtime_clone.clone(), username, password));
	main_window.on_save_settings(save_settings);
	main_window.on_load_settings(load_settings);
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_set_state(move|scene, host|set_state(game_clone.clone(), scene, host));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_add_player(move||add_player(game_clone.clone()));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_remove_player(move|index|remove_player(game_clone.clone(), index as u8));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_set_player_team(move|index, team|set_player_team(game_clone.clone(), index as u8, team));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_make_bid(move|bid|make_bid(game_clone.clone(), bid));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_play_card(move|card|play_card(game_clone.clone(), card));

	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_create_game(move|public|create_game(game_clone.clone(), public));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_join_game(move|code|join_game(game_clone.clone(), code));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_search_games(move|
		max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override
	|search_games(
		game_clone.clone(), max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override
	));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_send_game_meta(move|
		std_team_mode, win_by_2, negative_scores, dealer_override, max_score, max_players
	|send_game_meta(
		game_clone.clone(), std_team_mode, win_by_2, negative_scores, dealer_override, max_score as i8, max_players as u8
	));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_ready_up(move|ready|ready_up(game_clone.clone(), ready));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_logout(move||logout(game_clone.clone(), runtime.handle().clone()));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.on_set_player_type(move|index, human|set_player_type(game_clone.clone(), index as u8, human));

	info!("Setback game started.");

	main_window.run()
}

enum Log {
	Info(String),
	Warn(String),
	Error(String),
	Debug(String),
	Trace(String)
}

///Game state for the app
struct SetbackGame {
	scene_type: Scene,
	host_type: Host,
	connection: Option<ServerConnection>,
	event_tx: UnboundedSender<UiEvent>,
	local_player_index: u8
}

///Game state which stores what scene is currently active.
#[repr(u8)]
enum Scene {
	MainMenu,
	Setup,
	Multiplayer,
	Game,
	Settings
}

impl From<SceneType> for Scene {
	fn from(scene: SceneType) -> Self {
		match scene {
			SceneType::MainMenu => {Scene::MainMenu}
			SceneType::Setup => {Scene::Setup}
			SceneType::Multiplayer => {Scene::Multiplayer}
			SceneType::Game => {Scene::Game}
			SceneType::Settings => {Scene::Settings}
		}
	}
}

///Game state which stores the relation between the player and the type of game they're playing.
#[repr(u8)]
enum Host {
	///Not in a game.
	None,
	SingleplayerSetup(GameBuilder),
	SingleplayerPlaying(GameRuntime),
	///Created a multiplayer game as the host.
	//This indicates that this player can edit the game settings in the setup.
	//In game, this is the same as Client.
	Host,
	///Joined a multiplayer game as a client. Cannot edit the game settings in the setup.
	Client
}

#[repr(u8)]
enum UiEvent {
	PromptCredentials,
	ConnectionSuccess,
	AuthenticationError,
	UpdateTeam{index: u8, team: i32},
	UpdatePlayerType{index: u8, human: bool},
	UpdatePlayers(Vec<SetupPlayerData>),
	UpdatePublicGames(Vec<PublicGame>),
	UpdateGame(GameMeta),
	UnrecoverableError(String),
	RecoverableError(String, ErrorLocation),
	PromptBid,
	PromptCard,
	RepromptBid,
	RepromptCard,
	PlayerBid{index: u8, bid: u8},
	PlayerPlayedCard{index: u8, card: shared::Card},
	PlayerReadyUp{index: u8, ready: bool},
	PlayerTurn{index: u8},
	RoundScores(RoundScores),
	GameOver(Vec<i8>),
	DealCards,
	UpdateHand(Vec<shared::Card>),
	StartGame,
	EnterSetup{host: bool}
}

impl From<Comparison> for shared::Comparison {
	fn from(comparison: Comparison) -> Self {
		match comparison {
			Comparison::Equal => shared::Comparison::Equal,
			Comparison::LessOrEqual => shared::Comparison::LessOrEqual,
			Comparison::GreaterOrEqual => shared::Comparison::GreaterOrEqual,
			Comparison::Less => shared::Comparison::Less,
			Comparison::Greater => shared::Comparison::Greater,
			Comparison::NotEqual => shared::Comparison::NotEqual
		}
	}
}

impl From<Suit> for shared::Suit {
	fn from(suit: Suit) -> Self {
		match suit {
			Suit::Hearts => shared::Suit::Hearts,
			Suit::Clubs => shared::Suit::Clubs,
			Suit::Diamonds => shared::Suit::Diamonds,
			Suit::Spades => shared::Suit::Spades
		}
	}
}

impl From<Card> for shared::Card {
	fn from(card: Card) -> Self {
		shared::Card {suit: card.suit.into(), number: card.number as u8}
	}
}

impl Settings {
	fn new() -> Self {
		Settings {
			master_volume: 100.0,
			music_volume: 100.0,
			ui_volume: 100.0,
			resolution: SharedString::from("1920x1080"),
			framerate: SharedString::from("60 FPS"),
			vsync: true,
			triple_buffering: true,
			pass_keybind: SharedString::from("0"),
			bid_2_keybind: SharedString::from("2"),
			bid_3_keybind: SharedString::from("3"),
			bid_4_keybind: SharedString::from("4"),
			bid_5_keybind: SharedString::from("5"),
			play_card_1_keybind: SharedString::from("1"),
			play_card_2_keybind: SharedString::from("2"),
			play_card_3_keybind: SharedString::from("3"),
			play_card_4_keybind: SharedString::from("4"),
			play_card_5_keybind: SharedString::from("5"),
			play_card_6_keybind: SharedString::from("6"),
			toggle_settings_keybind: SharedString::from("esc")
		}
	}
}

impl Encode for Settings {
	fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
		Encode::encode(&self.master_volume, encoder)?;
		Encode::encode(&self.music_volume, encoder)?;
		Encode::encode(&self.ui_volume, encoder)?;
		Encode::encode(&self.resolution.to_string(), encoder)?;
		Encode::encode(&self.framerate.to_string(), encoder)?;
		Encode::encode(&self.vsync, encoder)?;
		Encode::encode(&self.triple_buffering, encoder)?;
		Encode::encode(&self.pass_keybind.to_string(), encoder)?;
		Encode::encode(&self.bid_2_keybind.to_string(), encoder)?;
		Encode::encode(&self.bid_3_keybind.to_string(), encoder)?;
		Encode::encode(&self.bid_4_keybind.to_string(), encoder)?;
		Encode::encode(&self.bid_5_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_1_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_2_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_3_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_4_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_5_keybind.to_string(), encoder)?;
		Encode::encode(&self.play_card_6_keybind.to_string(), encoder)?;
		Encode::encode(&self.toggle_settings_keybind.to_string(), encoder)
	}
}

impl Decode for Settings {
	fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
		Ok(Settings {
			master_volume: Decode::decode(decoder)?,
			music_volume: Decode::decode(decoder)?,
			ui_volume: Decode::decode(decoder)?,
			resolution: SharedString::from(<String as Decode>::decode(decoder)?),
			framerate: SharedString::from(<String as Decode>::decode(decoder)?),
			vsync: Decode::decode(decoder)?,
			triple_buffering: Decode::decode(decoder)?,
			pass_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			bid_2_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			bid_3_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			bid_4_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			bid_5_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_1_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_2_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_3_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_4_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_5_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			play_card_6_keybind: SharedString::from(<String as Decode>::decode(decoder)?),
			toggle_settings_keybind: SharedString::from(<String as Decode>::decode(decoder)?)
		})
	}
}

impl<'de> BorrowDecode<'de> for Settings {
	fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
		Ok(Settings {
			master_volume: BorrowDecode::borrow_decode(decoder)?,
			music_volume: BorrowDecode::borrow_decode(decoder)?,
			ui_volume: BorrowDecode::borrow_decode(decoder)?,
			resolution: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			framerate: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			vsync: BorrowDecode::borrow_decode(decoder)?,
			triple_buffering: BorrowDecode::borrow_decode(decoder)?,
			pass_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			bid_2_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			bid_3_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			bid_4_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			bid_5_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_1_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_2_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_3_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_4_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_5_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			play_card_6_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?),
			toggle_settings_keybind: SharedString::from(<String as BorrowDecode<'de>>::borrow_decode(decoder)?)
		})
	}
}

//Main menu functions

///Prompts user for credentials if necessary or creates connection to server with saved credentials.
fn multiplayer_clicked(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	{
		let game: RwLockReadGuard<SetbackGame> = setback_game.read().unwrap();
		if game.connection.is_some() {
			game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
			info!("Already connected to server.");
			return;
		}
	}
	info!("Looking for saved username.");
	let path: &Path = USERNAME_PATH.as_path();
	match fs::read_to_string(path) {
		Ok(username) => {
			info!("Found saved username, looking for password from secure storage.");
			match Entry::new(SERVICE, username.as_str()).unwrap().get_password() {
				Ok(password) => {
					info!("Found password in secure storage, connecting to server.");
					runtime.spawn(async move {
						match ServerConnection::new_connection(
							username,
							password,
							ssp_callback(setback_game.clone()),
							handle_error(setback_game.clone())
						).await {
							Ok(connection) => {
								info!("Connected to server.");
								let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
								game.connection = Some(connection);
								game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
							}
							Err(error) => {
								if let Error::Authentication = error {
									info!("Authentication error, prompting user for credentials.");
									setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
								} else {
									error!("Unexpected error connecting to server.");
									setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::MultiplayerPopup)).unwrap();
								}
							}
						}
					});
				}
				Err(keyring::Error::NoEntry) => {
					info!("No password found, prompting user for credentials.");
					File::create(path).unwrap();
					setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
				}
				Err(keyring::Error::Ambiguous(_)) => {
					error!("Ambiguous credentials. User may need to manually remove conflicting password from secure storage. Prompting user for credentials.");
					setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
				}
				Err(_) => {unreachable!()}
			}
		}
		Err(_) => {
			info!("No saved username found, prompting user for credentials.");
			File::create(path).unwrap();
			setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
		}
	}
}

///When the user submits their credentials, this function is called.
fn handle_credentials(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, username: SharedString, password: SharedString) {
	runtime.spawn(async move {
		info!("Attempting to connect to server with provided credentials.");
		let username: String = username.to_string();
		let password: String = password.to_string();
		match ServerConnection::new_connection(
			username.clone(),
			password.clone(),
			ssp_callback(setback_game.clone()),
			handle_error(setback_game.clone())
		).await {
			Ok(connection) => {
				info!("Connected to server.");
				let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
				game.connection = Some(connection);
				game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
				fs::write(USERNAME_PATH.as_path(), username.as_str()).unwrap();
				if let Err(keyring::Error::Ambiguous(_)) = Entry::new(SERVICE, username.as_str()).unwrap().set_password(password.as_str()) {
					error!("Ambiguous credentials. User may need to manually remove conflicting password from secure storage.");
				}
			}
			Err(error) => {
				if let Error::Authentication = error {
					info!("Authentication error, reprompting user for credentials.");
					setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
				} else {
					error!("Unexpected error connecting to server.");
					setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::MultiplayerPopup)).unwrap();
				}
			}
		}
	});
}

///Saves settings to disk
fn save_settings(settings: Settings) {
	bincode::encode_into_std_write(settings, &mut File::create(SETTINGS_PATH.as_path()).unwrap(), standard()).unwrap();
}

///Loads settings from disk
fn load_settings() -> Settings {
	match File::open(SETTINGS_PATH.as_path()) {
		Ok(mut file) => {
			bincode::decode_from_std_read(&mut file, standard()).unwrap()
		}
		Err(_) => {
			let settings: Settings = Settings::new();
			bincode::encode_into_std_write(&settings, &mut File::create(SETTINGS_PATH.as_path()).unwrap(), standard()).unwrap();
			settings
		}
	}
}

//singleplayer and multiplayer functions

///Sets the game state to the given scene and host.
fn set_state(setback_game: Arc<RwLock<SetbackGame>>, scene: SceneType, host: HostType) {
	info!("Setting scene to {:?} and host to {:?}.", scene, host);
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	match game.scene_type {
		Scene::MainMenu => {
			match scene {
				SceneType::Game => {unreachable!("You should not be able to change the scene to a game when you're in the main menu.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Setup => {
			match scene {
				SceneType::Multiplayer |
				SceneType::Settings => {unreachable!("You should not be able to change the scene to a multiplayer or settings when you're in the setup.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Multiplayer => {
			match scene {
				SceneType::Game |
				SceneType::Settings => {unreachable!("You should not be able to change the scene to a game or settings when you're in the multiplayer.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Game => {
			match scene {
				SceneType::Setup |
				SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the game.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Settings => {
			match scene {
				SceneType::Setup |
				SceneType::Multiplayer | SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the settings.")}
				_ => {game.scene_type = scene.into();}
			}
		}
	}
	match &game.host_type {
		Host::None => {
			match host {
				HostType::None | HostType::Host | HostType::Client => {}
				HostType::SingleplayerSetup => {
					//starting singleplayer game
					game.connection = None;
					game.host_type = Host::SingleplayerSetup(GameBuilder::default());
				}
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
		Host::SingleplayerSetup(_) => {
			match host {
				HostType::None => {game.host_type = Host::None;}//leaving the singleplayer game setup
				HostType::SingleplayerSetup => {}//no change in host
				HostType::SingleplayerPlaying => {
					//starting singleplayer game
					let Host::SingleplayerSetup(game_builder) = std::mem::replace(&mut game.host_type, Host::None) else {unreachable!()};
					let game_runtime: GameRuntime = game_builder.start_game();
					game.host_type = Host::SingleplayerPlaying(game_runtime);
				}
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
		Host::SingleplayerPlaying(game_runtime) => {
			match host {
				HostType::None => {
					//leaving the singleplayer game
					game_runtime.cancel_token.cancel();
					game.host_type = Host::None;
				}
				HostType::SingleplayerPlaying => {}//no change in host
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
		Host::Host => {
			match host {
				HostType::None => {
					//leaving the multiplayer game
					game.connection = None;
					game.host_type = Host::None;
				}
				HostType::Host => {}//no change in host
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
		Host::Client => {
			match host {
				HostType::None => {
					//leaving the multiplayer game
					game.connection = None;
					game.host_type = Host::None;
				}
				HostType::Client => {}//no change in host
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
	}
}

///Generic error handler.
fn handle_error(setback_game: Arc<RwLock<SetbackGame>>) -> impl Fn(Error) + Send + 'static {
	move |error: Error| {
		warn!("Received error from server: {:?}.", error);
		match error {
			Error::Indeterminate |
			Error::Authentication |
			Error::Authorization |
			Error::NetworkIO |
			Error::ServerError |
			Error::IncompatibleVersion |
			Error::InvalidProtocol |
			Error::GamesAtCapacity => {
				setback_game.write().unwrap().event_tx.send(UiEvent::UnrecoverableError(error.to_string())).unwrap();
			}
			Error::GameAtCapacity |
			Error::GameNotFound => {
				match setback_game.read().unwrap().scene_type {
					Scene::MainMenu => {setback_game.write().unwrap().event_tx.send(UiEvent::UnrecoverableError(error.to_string())).unwrap();}
					Scene::Multiplayer => {setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap();}
					_ => {unreachable!("Error in illegal scene.")}
				}
			}
			Error::InvalidBid |
			Error::InvalidCard |
			Error::OutOfTurn => {
				setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap();
			}
			Error::ClientNotFound => unreachable!("Server-only error"),
		}
	}
}

fn add_player(setback_game: Arc<RwLock<SetbackGame>>) {
	info!("Adding player.");
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.add_ai();
			let players: Vec<SetupPlayerData> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
			game.event_tx.send(UiEvent::UpdatePlayers(players)).unwrap();
		}
		Host::Host => {
			game.connection.as_mut().unwrap().send_add_player();
		}
		_ => unreachable!("Should not be able to add player")
	}
}

fn remove_player(setback_game: Arc<RwLock<SetbackGame>>, index: u8) {
	info!("Removing player at index {}.", index);
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.game.players.remove(index as usize);
			let players: Vec<SetupPlayerData> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
			game.event_tx.send(UiEvent::UpdatePlayers(players)).unwrap();
		}
		Host::Host => {
			//This will kick the human player from the game if one is connected
			game.connection.as_mut().unwrap().send_remove_player(index);
		}
		_ => unreachable!("Should not be able to remove player")
	}
}

fn set_player_team(setback_game: Arc<RwLock<SetbackGame>>, index: u8, team: i32) {
	info!("Setting player team at index {} to team {}.", index, team);
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.update_team(index, team as u8);
			game.event_tx.send(UiEvent::UpdateTeam{index, team}).unwrap();
		}
		Host::Host | Host::Client => {
			game.connection.as_mut().unwrap().send_update_team(index, team as u8);
		}
		_ => unreachable!("Should not be able to set player team")
	}
}

fn make_bid(setback_game: Arc<RwLock<SetbackGame>>, bid: i32) {
	info!("Making bid of {}.", bid);
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	match &mut game.host_type {
		Host::SingleplayerPlaying(game_runtime) => {
			game_runtime.prompt_senders[local_player_index as usize].0.try_send(bid as u8).unwrap();
			game.event_tx.send(UiEvent::PlayerBid{index: local_player_index, bid: bid as u8}).unwrap();
		}
		Host::Host | Host::Client => {
			game.connection.as_mut().unwrap().send_game_event(GameEvent::PlayerBid{index: local_player_index, amount: bid as u8});
		}
		_ => unreachable!("Should not be able to make bid")
	}
}

fn play_card(setback_game: Arc<RwLock<SetbackGame>>, card: Card) {
	info!("Playing card: {:?}.", card);
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	let card: shared::Card = card.into();
	match &mut game.host_type {
		Host::SingleplayerPlaying(game_runtime) => {
			game_runtime.prompt_senders[local_player_index as usize].1.try_send(card).unwrap();
			game.event_tx.send(UiEvent::PlayerPlayedCard{index: local_player_index, card}).unwrap();
		}
		Host::Host | Host::Client => {
			game.connection.as_mut().unwrap().send_game_event(GameEvent::PlayerPlayedCard{index: 0, card});
		}
		_ => unreachable!("Should not be able to play card")
	}
}

//multiplayer only functions

fn create_game(setback_game: Arc<RwLock<SetbackGame>>, public: bool) {
	info!("Creating game.");
	setback_game.write().unwrap().connection.as_mut().unwrap().send_create_game(public);
}

fn join_game(setback_game: Arc<RwLock<SetbackGame>>, code: SharedString) {
	info!("Joining game with code: {}.", code);
	setback_game.write().unwrap().connection.as_mut().unwrap().send_join_game(code.to_string().parse::<u32>().unwrap());
}

fn search_games(
	setback_game: Arc<RwLock<SetbackGame>>,
	max_score_cmp: Comparison,
	max_score_value: i32,
	max_players_cmp: Comparison,
	max_players_value: i32,
	std_team_mode: bool,
	win_by_2: bool,
	negative_scores: bool,
	dealer_override: bool
) {
	info!("Searching for public games.");
	setback_game.write().unwrap().connection.as_mut().unwrap().send_search_games(GameFilter {
		std_team_mode,
		win_by_2,
		negative_scores,
		dealer_override,
		max_score_cmp: shared::Comparison::from(max_score_cmp),
		max_score_value: max_score_value as i8,
		max_players_cmp: shared::Comparison::from(max_players_cmp),
		max_players_value: max_players_value as u8
	});
}

fn send_game_meta(
	setback_game: Arc<RwLock<SetbackGame>>,
	std_team_mode: bool,
	win_by_2: bool,
	negative_scores: bool,
	dealer_override: bool,
	max_score: i8,
	max_players: u8
) {
	info!("Sending game meta.");
	setback_game.write().unwrap().connection.as_mut().unwrap().send_game_meta(GameMeta {
		std_team_mode,
		win_by_2,
		negative_scores,
		dealer_override,
		max_score,
		max_players
	});
}

fn ready_up(setback_game: Arc<RwLock<SetbackGame>>, ready: bool) {
	info!("Readying up.");
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();	
	let local_player_index: u8 = game.local_player_index;
	game.connection.as_mut().unwrap().send_ready(ready, local_player_index);
}

fn logout(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	info!("Logging out.");
	setback_game.write().unwrap().connection.as_mut().unwrap().logout(runtime);
}

fn set_player_type(setback_game: Arc<RwLock<SetbackGame>>, index: u8, human: bool) {
	info!("Setting player type at index {} to {}.", index, if human { "human" } else { "AI" });
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	assert!(matches!(game.host_type, Host::Host), "Only the host can set player types.");
	game.connection.as_mut().unwrap().send_update_player_type(index, human);
}

///Ssp message received callback
fn ssp_callback(game: Arc<RwLock<SetbackGame>>) -> impl Fn(Option<Ssp>) + Send + Sync + 'static {
	move |message: Option<Ssp>| {
		let mut game: RwLockWriteGuard<SetbackGame> = game.write().unwrap();
		match message {
			Some(ssp) => {
				info!("Received message from server: {:?}.", ssp);
				match ssp.protocol {
					Protocol::SyncID => {}
					Protocol::Err(error) => {
						match error {
							Error::InvalidBid => game.event_tx.send(UiEvent::RepromptBid).unwrap(),
							Error::InvalidCard => game.event_tx.send(UiEvent::RepromptCard).unwrap(),
							_ => game.event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap()
						}
					}
					Protocol::Game(game_meta, player_datas, host) => {
						game.event_tx.send(UiEvent::EnterSetup{host}).unwrap();
						game.event_tx.send(UiEvent::UpdateGame(game_meta)).unwrap();
						game.event_tx.send(UiEvent::UpdatePlayers(as_ui_player_vec(game.local_player_index, player_datas.as_slice()))).unwrap();
					}
					Protocol::PublicGames(public_games) => game.event_tx.send(UiEvent::UpdatePublicGames(public_games)).unwrap(),
					Protocol::GameMeta(game_meta) => game.event_tx.send(UiEvent::UpdateGame(game_meta)).unwrap(),
					Protocol::UpdatePlayerTeam{index, team} => game.event_tx.send(UiEvent::UpdateTeam{index, team: team as i32}).unwrap(),
					Protocol::UpdatePlayerType{index, human} => game.event_tx.send(UiEvent::UpdatePlayerType{index, human}).unwrap(),
					Protocol::Players{players, local_player_index} => game.event_tx.send(UiEvent::UpdatePlayers(as_ui_player_vec(local_player_index, players.as_slice()))).unwrap(),
					Protocol::Ready{is_ready, index} => game.event_tx.send(UiEvent::PlayerReadyUp{index, ready: is_ready}).unwrap(),
					Protocol::Start => game.event_tx.send(UiEvent::StartGame).unwrap(),
					Protocol::GameEvent(game_event) => {
						match game_event {
							GameEvent::PlayerBid{index, amount} => game.event_tx.send(UiEvent::PlayerBid{index, bid: amount}).unwrap(),
							GameEvent::PlayerPlayedCard{index, card} => game.event_tx.send(UiEvent::PlayerPlayedCard{index, card}).unwrap(),
							GameEvent::PlayerTurn{index} => game.event_tx.send(UiEvent::PlayerTurn{index}).unwrap(),
							GameEvent::PromptBid{hand,..} => {
								game.event_tx.send(UiEvent::UpdateHand(hand)).unwrap();
								game.event_tx.send(UiEvent::PromptBid).unwrap();
							}
							GameEvent::PromptCard{hand,..} => {
								game.event_tx.send(UiEvent::UpdateHand(hand)).unwrap();
								game.event_tx.send(UiEvent::PromptCard).unwrap();
							}
							GameEvent::DealCards => game.event_tx.send(UiEvent::DealCards).unwrap(),
							GameEvent::RoundScores(round_scores) => game.event_tx.send(UiEvent::RoundScores(round_scores)).unwrap(),
							GameEvent::GameOver(team_scores) => game.event_tx.send(UiEvent::GameOver(team_scores)).unwrap(),
							GameEvent::GameError(error) => game.event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::Scene)).unwrap(),
						}
					}
					Protocol::Disconnect => unreachable!("Server doesn't send this to clients"),
					Protocol::Admin(_) => unreachable!("Clients are not permitted to send or receive admin messages."),
					_ => unreachable!("Request only")
				}
			}
			None => {
				info!("Connection lost.");
				game.connection = None;
				game.event_tx.send(UiEvent::UnrecoverableError(String::from("Connection lost."))).unwrap();
			}
		}
	}
}

//Util functions

const ELLIPSE_RADIUS: f32 = 525.;
const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

fn as_ui_player_vec(local_player_index: u8, players: &[impl AsRef<PlayerData>]) -> Vec<SetupPlayerData> {
	let mut players: Vec<SetupPlayerData> = players
		.iter()
		.enumerate()
		.map(|(index, player)| {
			let theta: f32 = 2. * PI * index as f32 / players.len() as f32 + 3. * PI / 2.;
			SetupPlayerData {
				local: false,
				is_human: !matches!(player.as_ref().player_type, PlayerType::AI),
				team: player.as_ref().team_id as i32,
				username: SharedString::from(&player.as_ref().username),
				ready: match player.as_ref().player_type {
					PlayerType::MultiplayerHuman{ready,..} => ready,
					_ => true
				},
				x: -theta.cos() * ELLIPSE_RADIUS,
				y: -ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS
			}
		})
		.collect::<Vec<SetupPlayerData>>();
	players[local_player_index as usize].local = true;
	players
}