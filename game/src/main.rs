#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	shared::{
		game::{GameBuilder, GameRuntime},
		ServerConnection, LoginCredentials, GameEvent, PlayerType, GameMeta, Error, Ssp, GameFilter
	},
	slint::{ModelRc, SharedString, Weak},
	std::{
		env,
		f32::consts::PI,
		fs::{self, File},
		path::{PathBuf, Path},
		sync::{Arc, LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard}
	},
	tokio::{
		runtime::{Handle, Runtime},
		sync::mpsc::{unbounded_channel, UnboundedSender}
	},
	bincode::config::standard
};

slint::include_modules!();

static PROJ_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	env::current_exe()
		.ok()
		.and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
		.unwrap()
});

static CREDENTIALS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join(".cred")
});

static SETTINGS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join(".config")
});

static LOG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
	PROJ_DIR.join("log.txt")
});

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
			match event {
				// window_weak.upgrade_in_event_loop(move |window| {}); //this is how you update the ui from other threads/tasks
				UiEvent::PromptCredentials => {}
				UiEvent::ConnectionError(error) => {}
				UiEvent::ConnectionSuccess => {}
				UiEvent::AuthenticationError => {}
				UiEvent::UpdateTeam{index, team} => {}
				UiEvent::UpdatePlayerType {index, human} => {}
				UiEvent::UpdatePlayers(players) => {}
				UiEvent::UpdatePublicGames(game_metas) => {}
				UiEvent::UpdateGame(game_meta) => {}
				UiEvent::UnrecoverableError(error) => {
					window_weak.upgrade_in_event_loop(move |window| {
						window.set_had_error(true);
						window.set_error_message(SharedString::from(&error));
						window.global::<GameState>().invoke_set_state(SceneType::MainMenu, HostType::None);
					}).unwrap();
				},
				UiEvent::MultiplayerError(error) => {}
				UiEvent::GameError(error) => {}
				UiEvent::PromptBid => {}
				UiEvent::PromptCard => {}
				UiEvent::RepromptBid => {}
				UiEvent::RepromptCard => {}
				UiEvent::PlayerBid {index, bid} => {}
				UiEvent::PlayerPlayedCard {index, card} => {}
				UiEvent::PlayerReadyUp {index, ready} => {}
			}
		}
	});

	//UI event handlers
	main_window.window().set_maximized(true);

	main_window.global::<GameState>().on_exit(||std::process::exit(0));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_set_state(move|scene, host|set_state(game_clone.clone(), runtime.handle().clone(), scene, host));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_make_bid(move|bid|make_bid(game_clone.clone(), bid));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_play_card(move|card|play_card(game_clone.clone(), card));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_create_game(move|public|create_game(game_clone.clone(), public));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_join_game(move|code|join_game(game_clone.clone(), code));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_search_games(move|
		max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override
	|search_games(
		game_clone.clone(), max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override
	));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_get_game_meta(move||get_game_meta(game_clone.clone()));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_ready_up(move|ready|ready_up(game_clone.clone(), ready));

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
	ConnectionError(String),
	ConnectionSuccess,
	AuthenticationError,
	UpdateTeam{index: u8, team: i32},
	UpdatePlayerType{index: u8, human: bool},
	UpdatePlayers(Vec<Player>),
	UpdatePublicGames(Vec<GameMeta>),
	UpdateGame(GameMeta),
	UnrecoverableError(String),
	MultiplayerError(String),
	GameError(String),
	PromptBid,
	PromptCard,
	RepromptBid,
	RepromptCard,
	PlayerBid{index: u8, bid: u8},
	PlayerPlayedCard{index: u8, card: shared::Card},
	PlayerReadyUp{index: u8, ready: bool}
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

//Main menu functions

///Prompts user for credentials if necessary or creates connection to server with saved credentials.
fn multiplayer_clicked(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	let path: &Path = CREDENTIALS_PATH.as_path();
	if path.exists() {
		match fs::read(path) {
			Ok(contents) => {
				match bincode::decode_from_slice::<LoginCredentials, _>(contents.as_slice(), standard()) {
					Ok((credentials, _)) => {
						//file read successfully
						runtime.spawn(async move {
							match ServerConnection::new_connection(
								credentials.username,
								credentials.password,
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
										setback_game.write().unwrap().event_tx.send(UiEvent::ConnectionError(error.to_string())).unwrap();
									}
								}
							}
						});
					}
					Err(_) => {
						//failed to decode credentials
						//override the file with a new empty one
						File::create(path).unwrap();
						setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
					}
				}
			}
			Err(_) => {
				//failed to read file
				setback_game.write().unwrap().event_tx.send(UiEvent::ConnectionError(String::from("Could not load credentials. Make sure the .cred file can be opened and read, then try again."))).unwrap();
			}
		}
	} else {
		//create the dir/file
		fs::create_dir_all(PROJ_DIR.as_path()).unwrap();
		File::create(path).unwrap();
		setback_game.write().unwrap().event_tx.send(UiEvent::PromptCredentials).unwrap();
	}
}

///When the user submits their credentials, this function is called.
fn handle_credentials(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, username: SharedString, password: SharedString) {
	runtime.spawn(async move {
		match ServerConnection::new_connection(
			username.to_string(),
			password.to_string(),
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
					setback_game.write().unwrap().event_tx.send(UiEvent::ConnectionError(error.to_string())).unwrap();
				}
			}
		}
	});
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
					Scene::Multiplayer => {setback_game.write().unwrap().event_tx.send(UiEvent::MultiplayerError(error.to_string())).unwrap();}
					_ => {unreachable!("Error in illegal scene.")}
				}
			}
			Error::InvalidBid |
			Error::InvalidCard |
			Error::OutOfTurn => {
				setback_game.write().unwrap().event_tx.send(UiEvent::GameError(error.to_string())).unwrap();
			}
			Error::ClientNotFound => unreachable!("Server-only error"),
		}
	}
}

fn add_player(setback_game: Arc<RwLock<SetbackGame>>) {
	let mut game: RwLockWriteGuard<SetbackGame> = setback_game.write().unwrap();
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.add_ai();
			let players: Vec<Player> = as_ui_player_vec(game_builder.game.players.as_slice());
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
	match &mut game.host_type {
		Host::SingleplayerSetup(game_builder) => {
			game_builder.game.players.remove(index as usize);
			let players: Vec<Player> = as_ui_player_vec(game_builder.game.players.as_slice());
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

fn get_game_meta(setback_game: Arc<RwLock<SetbackGame>>) {
	todo!("game meta params from UI");
	// setback_game.write().unwrap().connection.as_mut().unwrap().send_game_meta();
}

fn ready_up(setback_game: Arc<RwLock<SetbackGame>>, ready: bool) {
	setback_game.write().unwrap().connection.as_mut().unwrap().send_ready(ready);
}

fn logout(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle) {
	setback_game.write().unwrap().connection.as_mut().unwrap().logout(runtime);
}

fn kick_player(setback_game: Arc<RwLock<SetbackGame>>, index: u8) {
	set_player_type(setback_game, index, false);
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
				todo!()
			}
			None => {
				game.connection = None;
				game.event_tx.send(UiEvent::UnrecoverableError(String::from("Connection lost."))).unwrap();
			}
		}
	}
}

//Util functions

fn as_ui_player_vec(players: &[shared::Player]) -> Vec<Player> {
	const ELLIPSE_RADIUS: f32 = 525.;
	const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

	players.iter()
		.enumerate()
		.map(|(index, player)| {
			let theta: f32 = 2. * PI * index as f32 / players.len() as f32 + 3. * PI / 2.;
			Player {
				local: false,
				is_human: !matches!(player.player_data.player_type, PlayerType::AI),
				team: player.player_data.team_id as i32,
				username: SharedString::from(&player.player_data.username),
				x: theta.cos() * ELLIPSE_RADIUS,
				y: ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS
			}
		})
		.collect::<Vec<Player>>()
}