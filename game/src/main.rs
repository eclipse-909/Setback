#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	shared::{
		game::{GameBuilder, GameRuntime},
		ServerConnection
	},
	slint::{ModelRc, SharedString, Weak},
	std::{
		f32::consts::PI,
		sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}
	},
	tokio::{
		runtime::{Runtime, Handle},
		sync::mpsc::{unbounded_channel, UnboundedSender}
	}
};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
	let main_window: MainWindow = MainWindow::new()?;
	let runtime: Runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();
	let game: Arc<RwLock<SetbackGame>> = SetbackGame::new(main_window.as_weak(), runtime.handle().clone());
	main_window.window().set_maximized(true);

	main_window.global::<GameState>().on_exit(on_exit);
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_set_state(move|scene, host|on_set_state(game_clone.clone(), runtime.handle().clone(), scene, host));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_make_bid(move|bid|on_make_bid(game_clone.clone(), bid));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_play_card(move|card|on_play_card(game_clone.clone(), card));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_create_game(move|public|on_create_game(game_clone.clone(), public));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_join_game(move|code|on_join_game(game_clone.clone(), code));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_search_games(move|max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override|on_search_games(game_clone.clone(), max_score_cmp, max_score_value, max_players_cmp, max_players_value, std_team_mode, win_by_2, negative_scores, dealer_override));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_get_game_meta(move||on_get_game_meta(game_clone.clone()));
	let game_clone: Arc<RwLock<SetbackGame>> = game.clone();
	main_window.global::<GameState>().on_ready_up(move|ready|on_ready_up(game_clone.clone(), ready));

	main_window.run()
}

///Game state for the app
struct SetbackGame {
	scene_type: Scene,
	host_type: Host,
	connection: Option<ServerConnection>,
	event_tx: UnboundedSender<UiEvent>
}

unsafe impl Send for SetbackGame {}
unsafe impl Sync for SetbackGame {}

impl SetbackGame {
	fn new(window_weak: Weak<MainWindow>, runtime: Handle) -> Arc<RwLock<SetbackGame>> {
		let (event_tx, mut event_rx) = unbounded_channel();
		let setback_game: Arc<RwLock<SetbackGame>> = Arc::new(RwLock::new(SetbackGame {
			scene_type: Scene::default(),
			host_type: Host::default(),
			connection: None,
			event_tx
		}));
		let setback_game_clone: Arc<RwLock<SetbackGame>> = setback_game.clone();
		let runtime_clone: Handle = runtime.clone();
		runtime.spawn(async move {
			while let Some(event) = event_rx.recv().await {
				match event {
					// window_weak.upgrade_in_event_loop(move |window| {}); //this is how you update the ui from other threads/tasks

					UiEvent::UpdatePlayers(players) => todo!(),
					UiEvent::UpdatePublicGames(game_metas) => todo!(),
					UiEvent::UpdateGame(game_meta) => todo!(),
					UiEvent::UnrecoverableError(_) => {
						todo!("Set error in UI");
						on_set_state(setback_game_clone, runtime_clone, SceneType::MainMenu, HostType::None);
					},
					UiEvent::MultiplayerError(_) => todo!(),
					UiEvent::SetupError(_) => todo!(),
					UiEvent::GameError(_) => todo!(),
					UiEvent::PromptBid => todo!(),
					UiEvent::PromptCard => todo!(),
					UiEvent::RepromptBid => todo!(),
					UiEvent::RepromptCard => todo!(),
					UiEvent::PlayerBid { index, bid } => todo!(),
					UiEvent::PlayerPlayedCard { index, card } => todo!(),
					UiEvent::PlayerReadyUp { index, ready } => todo!(),
					
				}
			}
		});
		setback_game
	}
}

///Game state which stores what scene is currently active.
#[derive(Default)]
#[repr(u8)]
enum Scene {
	#[default]
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
#[derive(Default)]
#[repr(u8)]
enum Host {
	///Not in a game.
	#[default]
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
	UpdatePlayers(Vec<shared::Player>),
	UpdatePublicGames(Vec<shared::GameMeta>),
	UpdateGame(shared::GameMeta),
	UnrecoverableError(String),
	MultiplayerError(String),
	SetupError(String),
	GameError(String),
	PromptBid,
	PromptCard,
	RepromptBid,
	RepromptCard,
	PlayerBid{index: u8, bid: u8},
	PlayerPlayedCard{index: u8, card: shared::Card},
	PlayerReadyUp{index: u8, ready: bool},
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

//singleplayer and multiplayer functions

#[inline(always)]
fn on_exit() {std::process::exit(0);}

fn on_set_state(setback_game: Arc<RwLock<SetbackGame>>, runtime: Handle, scene: SceneType, host: HostType) {
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
					HostType::None => {}//no change in host
					HostType::SingleplayerSetup => {game.host_type = Host::SingleplayerSetup(GameBuilder::default());}//starting singleplayer game
					HostType::Host => {
						//starting multiplayer game
						
						//TODO show loading screen or message that says "connecting to server..."

						todo!("Load username and password from disk, or prompt user");
						let username: &str = todo!();
						let password: &str = todo!();
						match ServerConnection::new_connection(username, password, ssp_callback(setback_game), on_error(setback_game)).await {
							Some(connection) => {
								game.connection = Some(connection);
								todo!();
							}
							None => {
								todo!("connection failed");
							}
						}
					}
					HostType::Client => {
						//joining multiplayer game
						
						//TODO show loading screen or message that says "connecting to server..."
						
						todo!("Load username and password from disk, or prompt user");
						let username: &str = todo!();
						let password: &str = todo!();
						match ServerConnection::new_connection(username, password, ssp_callback(setback_game), on_error(setback_game)).await {
							Some(connection) => {
								game.connection = Some(connection);
								todo!();
							}
							None => {
								todo!("connection failed");
							}
						}
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
						let Host::SingleplayerSetup(game_builder) = std::mem::take(&mut game.host_type) else {unreachable!()};
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
						let connection = game.connection.take().unwrap();
						tokio::task::spawn_local(connection.shutdown()); //TODO don't do this
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
						let connection = game.connection.take().unwrap();
						tokio::task::spawn_local(connection.shutdown()); //TODO don't do this
						game.host_type = Host::None;
					}
					HostType::Client => {}//no change in host
					_ => {unreachable!("Illegal change in host type.")}
				}
			}
		}
	});
}

fn on_error(setback_game: Arc<RwLock<SetbackGame>>) -> impl Fn(shared::Error) + Send + 'static {
	move |error: shared::Error| {
		match error {
			shared::Error::Indeterminate |
			shared::Error::Authentication |
			shared::Error::Authorization |
			shared::Error::NetworkIO |
			shared::Error::IncompatibleVersion |
			shared::Error::InvalidProtocol => {
				setback_game.write().unwrap().event_tx.send(UiEvent::UnrecoverableError(error.to_string())).unwrap();
			}
			_ => {}
		}
	}
}

fn on_add_player(setback_game: Arc<RwLock<SetbackGame>>) {todo!()}
fn on_remove_player(setback_game: Arc<RwLock<SetbackGame>>, index: u8) {todo!()}
fn on_set_player_team(setback_game: Arc<RwLock<SetbackGame>>, index: u8, team: i32) {todo!()}
fn on_make_bid(setback_game: Arc<RwLock<SetbackGame>>, bid: i32) {todo!()}
fn on_play_card(setback_game: Arc<RwLock<SetbackGame>>, card: Card) {todo!()}

//multiplayer only functions

fn on_create_game(setback_game: Arc<RwLock<SetbackGame>>, public: bool) {setback_game.write().unwrap().connection.as_mut().unwrap().send_create_game(public);}
fn on_join_game(setback_game: Arc<RwLock<SetbackGame>>, code: slint::SharedString) {setback_game.write().unwrap().connection.as_mut().unwrap().send_join_game(code.to_string().parse::<u32>().unwrap());}
fn on_search_games(
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
	setback_game.write().unwrap().connection.as_mut().unwrap().send_search_games(shared::GameFilter {
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
fn on_get_game_meta(setback_game: Arc<RwLock<SetbackGame>>) {
	todo!("get game meta from UI");
	// setback_game.write().unwrap().connection.as_mut().unwrap().send_game_meta();
}
fn on_ready_up(setback_game: Arc<RwLock<SetbackGame>>, ready: bool) {setback_game.write().unwrap().connection.as_mut().unwrap().send_ready(ready);}
fn on_logout(setback_game: Arc<RwLock<SetbackGame>>) {todo!()}

///Ssp message received callback
fn ssp_callback(game: Arc<RwLock<SetbackGame>>) -> fn(message: Result<shared::Ssp, shared::Error>) {
	move |message: Result<shared::Ssp, shared::Error>| {
		match message {
			Ok(ssp) => {
				todo!()
			}
			Err(error) => {
				todo!()
			}
		}
	}
}

//only used in singleplayer

///Fetches the players from the singleplayer setup and displays them in the UI.
fn on_get_players(setback_game: Arc<RwLock<SetbackGame>>, main_window: &MainWindow) {
	const ELLIPSE_RADIUS: f32 = 525.;
	const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

	let game: RwLockReadGuard<SetbackGame> = setback_game.read().unwrap();
	let Host::SingleplayerSetup(game_builder) = &game.host_type else {unreachable!("Cannot fetch players outside of singleplayer setup.")};
	let players: &[shared::Player] = game_builder.game.players.as_slice();
	let players = ModelRc::from(
		players.iter()
			.enumerate()
			.map(|(index, player)| {
				let theta: f32 = 2. * PI * index as f32 / players.len() as f32 + 3. * PI / 2.;
				Player {
					local: false,
					is_human: !matches!(player.player_data.player_type, shared::PlayerType::AI),
					team: player.player_data.team_id as i32,
					username: SharedString::from(&player.player_data.username),
					x: theta.cos() * ELLIPSE_RADIUS,
					y: ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS
				}
			})
			.collect::<Vec<Player>>()
			.as_slice()
	);
	main_window.global::<GameState>().set_players(players);
}