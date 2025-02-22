#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	shared::{
		game::{GameBuilder, GameRuntime},
		ServerConnection, GameEvent, PlayerType, GameMeta, Error, Ssp, GameFilter, Protocol, PlayerData, RoundScores, PublicGame
	},
	slint::{ModelRc, SharedString, Weak},
	std::{
		env,
		f32::consts::PI,
		fs::{self, File},
		path::{PathBuf, Path},
		sync::{Arc, LazyLock, RwLock, RwLockWriteGuard}
	},
	tokio::{
		runtime::{Handle, Runtime},
		sync::mpsc::{unbounded_channel, UnboundedSender}
	},
	bincode::config::standard,
	keyring::Entry
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

static LOG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join("log.txt")
});

const SERVICE: &str = "com.github.setback-game.setback";

fn main() -> Result<(), slint::PlatformError> {
	//Tokio runtime
	let runtime: Runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

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
					UiEvent::PromptCredentials => {}
					UiEvent::ConnectionSuccess => {}
					UiEvent::AuthenticationError => {}
					UiEvent::UpdateTeam{index, team} => {}
					UiEvent::UpdatePlayerType {index, human} => {}
					UiEvent::UpdatePlayers(players) => {
						window.set_players(ModelRc::from(players.as_slice()));
					}
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
					UiEvent::PlayerReadyUp{index, ready} => {}
					UiEvent::PlayerTurn {index} => {}
					UiEvent::RoundScores(round_scores) => {}
					UiEvent::GameOver(items) => {}
					UiEvent::DealCards => {}
					UiEvent::UpdateHand(hand) => {}
					UiEvent::StartGame => {}
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
	let runtime_clone: Handle = runtime.handle().clone();
	main_window.on_set_state(move|scene, host|set_state(game_clone.clone(), runtime_clone.clone(), scene, host));
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

	main_window.run()
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
	///Created a multiplayer game as the host. This indicates that this player can edit the game settings in the setup. In game, this is the same as Client.
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
	UpdatePlayers(Vec<Player>),
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

impl Default for Settings {
	fn default() -> Self {
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

//Main menu functions

///Prompts user for credentials if necessary or creates connection to server with saved credentials.
fn multiplayer_clicked(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	let path: &Path = USERNAME_PATH.as_path();
	match fs::read_to_string(path) {
		Ok(username) => {
			match Entry::new(SERVICE, username.as_str()).unwrap().get_password() {
				Ok(password) => {
					runtime.spawn(async move {
						match ServerConnection::new_connection(
							username,
							password,
							ssp_callback(setback_game.clone()),
							handle_error(setback_game.clone())
						).await {
							Ok(connection) => {
								let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
								game.connection = Some(connection);
								game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
							}
							Err(error) => {
								if let Error::Authentication = error {
									setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
								} else {
									setback_game.write().unwrap().event_tx.send(UiEvent::RecoverableError(error.to_string(), ErrorLocation::MultiplayerPopup)).unwrap();
								}
							}
						}
					});
				}
				Err(keyring::Error::NoEntry) => {
					File::create(path).unwrap();
					setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
				}
				Err(keyring::Error::Ambiguous(_)) => {panic!("Ambiguous credentials")}//TODO handle error
				Err(_) => {unreachable!()}
			}
		}
		Err(_) => {
			File::create(path).unwrap();
			setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
		}
	}
}

///When the user submits their credentials, this function is called.
fn handle_credentials(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, username: SharedString, password: SharedString) {
	runtime.spawn(async move {
		let username: String = username.to_string();
		let password: String = password.to_string();
		match ServerConnection::new_connection(
			username.clone(),
			password.clone(),
			ssp_callback(setback_game.clone()),
			handle_error(setback_game.clone())
		).await {
			Ok(connection) => {
				let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
				game.connection = Some(connection);
				game.event_tx.send(UiEvent::ConnectionSuccess).unwrap();
				fs::write(USERNAME_PATH.as_path(), username.as_str()).unwrap();
				if let Err(keyring::Error::Ambiguous(_)) = Entry::new(SERVICE, username.as_str()).unwrap().set_password(password.as_str()) {
					panic!("Ambiguous credentials");
				}
			}
			Err(error) => {
				if let Error::Authentication = error {
					setback_game.write().unwrap().event_tx.send(UiEvent::AuthenticationError).unwrap();
				} else {
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
	match &mut File::open(SETTINGS_PATH.as_path()) {
		Ok(mut file) => {
			bincode::decode_from_std_read(&mut file, standard()).unwrap()
		}
		Err(_) => {
			let settings: Settings = Settings::default();
			save_settings(settings);
			settings
		}
	}
}

//singleplayer and multiplayer functions

///Sets the game state to the given scene and host.
fn set_state(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, scene: SceneType, host: HostType) {
	runtime.spawn(async move {
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
	});
}

///Generic error handler.
fn handle_error(setback_game: Arc<RwLock<SetbackGame>>) -> impl Fn(Error) + Send + 'static {
	move |error: Error| {
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
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.add_ai();
			let players: Vec<Player> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
			game.event_tx.send(UiEvent::UpdatePlayers(players)).unwrap();
		}
		Host::Host => {
			game.connection.as_mut().unwrap().send_add_player();
		}
		_ => unreachable!("Should not be able to add player")
	}
}

fn remove_player(setback_game: Arc<RwLock<SetbackGame>>, index: u8) {
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	let local_player_index: u8 = game.local_player_index;
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.game.players.remove(index as usize);
			let players: Vec<Player> = as_ui_player_vec(local_player_index, game_builder.game.players.as_slice());
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
	setback_game.write().unwrap().connection.as_mut().unwrap().send_create_game(public);
}

fn join_game(setback_game: Arc<RwLock<SetbackGame>>, code: SharedString) {
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
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();	
	let local_player_index: u8 = game.local_player_index;
	game.connection.as_mut().unwrap().send_ready(ready, local_player_index);
}

fn logout(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	setback_game.write().unwrap().connection.as_mut().unwrap().logout(runtime);
}

fn set_player_type(setback_game: Arc<RwLock<SetbackGame>>, index: u8, human: bool) {
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
				game.connection = None;
				game.event_tx.send(UiEvent::UnrecoverableError(String::from("Connection lost."))).unwrap();
			}
		}
	}
}

//Util functions

fn as_ui_player_vec(local_player_index: u8, players: &[impl AsRef<PlayerData>]) -> Vec<Player> {
	const ELLIPSE_RADIUS: f32 = 525.;
	const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

	let mut players: Vec<Player> = players
		.iter()
		.enumerate()
		.map(|(index, player)| {
			let theta: f32 = 2. * PI * index as f32 / players.len() as f32 + 3. * PI / 2.;
			Player {
				local: false,
				is_human: !matches!(player.as_ref().player_type, PlayerType::AI),
				team: player.as_ref().team_id as i32,
				username: SharedString::from(&player.as_ref().username),
				x: theta.cos() * ELLIPSE_RADIUS,
				y: ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS
			}
		})
		.collect::<Vec<Player>>();
	players[local_player_index as usize].local = true;
	players
}