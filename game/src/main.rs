#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	shared::{
		game::{GameBuilder, GameRuntime},
		ServerConnection
	},
	std::{
		f32::consts::PI,
		rc::Rc,
		cell::{
			RefCell,
			RefMut
		}
	},
	slint::{ModelRc, SharedString}
};

slint::include_modules!();

macro_rules! set_callbacks {
	($main_window:ident, $setback_game:ident, {$($callback:ident $(< $game:ident)? $(< ($($arg:ident),*))?),*}) => {
	    $(set_callbacks!($main_window, $callback $(, $game)? $(, ($($arg),*))?));*
    };
	($main_window:ident, $callback:ident) => {
		//no args
		$main_window.global::<GameState>().$callback($callback);
	};
	($main_window:ident, $callback:ident, $game:ident) => {
		//capture game
		$main_window.global::<GameState>().$callback({let g:Rc<RefCell<SetbackGame>>=$game.clone();move||$callback(g.clone())});
	};
	($main_window:ident, $callback:ident, ($($arg:ident),*)) => {
		//args
		$main_window.global::<GameState>().$callback(|$($arg),*|$callback($($arg),*));
	};
	($main_window:ident, $callback:ident, $game:ident, ($($arg:ident),*)) => {
		//capture game and args
		$main_window.global::<GameState>().$callback({let g:Rc<RefCell<SetbackGame>>=$game.clone();move|$($arg),*|$callback(g.clone(), $($arg),*)});
	};
}

//ellipse for player positions in setup
const ELLIPSE_RADIUS: f32 = 525.;
const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
	let game: Rc<RefCell<SetbackGame>> = Rc::new(RefCell::new(SetbackGame::default()));
	let main_window: MainWindow = MainWindow::new()?;
	main_window.window().set_maximized(true);
	set_callbacks!(main_window, game, {
		on_exit,
		on_set_state < game < (scene, host),
		on_bid < game < (bid),
		on_play_card < game < (card),
		on_send_create_game < game,
		on_send_join < game < (code),
		on_send_search_games < game < (filter),
		on_send_game_meta < game,
		on_send_player < game < (player),
		on_send_players < game < (players),
		on_send_ready < game < (ready),
		on_logout < game,
		on_get_account < game,
		on_fetch_players < game,
		on_fetch_games < game,
		on_calc_player_pos < game < (index)
	});
	main_window.run()
}

///Game state for the app
#[derive(Default)]
struct SetbackGame {
	scene_type: Scene,
	host_type: Host,
	connection: Option<ServerConnection>
}

impl SetbackGame {
	fn fetch_players(&self) -> &[shared::Player] {
		match &self.host_type {
			Host::SingleplayerSetup(game_builder) => {
				game_builder.game.players.as_slice()
			}
			Host::Host | Host::Client => {todo!("Send request to server to fetch players.")}
			_ => {unreachable!("Illegal host type.")}
		}
	}
	fn fetch_num_players(&self) -> u8 {
		match &self.host_type {
			Host::SingleplayerSetup(game_builder) => {game_builder.game.players.len() as u8}
			Host::Host | Host::Client => {todo!("Send request to server to fetch the number of players.")}
			_ => {unreachable!("Illegal host type.")}
		}
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

fn on_exit() {std::process::exit(0);}
fn on_set_state(setback_game: Rc<RefCell<SetbackGame>>, scene: SceneType, host: HostType) {
	let mut game: RefMut<SetbackGame> = setback_game.borrow_mut();
	match &mut game.scene_type {
		Scene::MainMenu => {
			match scene {
				SceneType::Game => {unreachable!("You should not be able to change the scene to a game when you're in the main menu.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Setup => {
			match scene {
				SceneType::Multiplayer | SceneType::Settings => {unreachable!("You should not be able to change the scene to a multiplayer or settings when you're in the setup.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Multiplayer => {
			match scene {
				SceneType::Game | SceneType::Settings => {unreachable!("You should not be able to change the scene to a game or settings when you're in the multiplayer.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Game => {
			match scene {
				SceneType::Setup | SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the game.")}
				_ => {game.scene_type = scene.into();}
			}
		}
		Scene::Settings => {
			match scene {
				SceneType::Setup | SceneType::Multiplayer | SceneType::Game => {unreachable!("You should not be able to change the scene to a setup or game when you're in the settings.")}
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
					
					let game_clone: Rc<RefCell<SetbackGame>> = setback_game.clone();
					tokio::task::spawn_local(async move {
						todo!("Load username and password from disk, or prompt user");
						let username: &str = todo!();
						let password: &str = todo!();
						match ServerConnection::new_connection(username, password, ssp_callback(game_clone)).await {
							Some(connection) => {
								game_clone.borrow_mut().connection = Some(connection);
								todo!()
							}
							None => {
								todo!("connection failed")
							}
						}
					});
				}
				HostType::Client => {
					//joining multiplayer game
					
					//TODO show loading screen or message that says "connecting to server..."
					
					let game_clone: Rc<RefCell<SetbackGame>> = setback_game.clone();
					tokio::task::spawn_local(async move {
						todo!("Load username and password from disk, or prompt user");
						let username: &str = todo!();
						let password: &str = todo!();
						match ServerConnection::new_connection(username, password, ssp_callback(game_clone)).await {
							Some(connection) => {
								// game.host_type = Host::Host(connection);
								todo!()
							}
							None => {
								todo!()
							}
						}
					});
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
					tokio::task::spawn_local(connection.shutdown());
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
					tokio::task::spawn_local(connection.shutdown());
					game.host_type = Host::None;
				}
				HostType::Client => {}//no change in host
				_ => {unreachable!("Illegal change in host type.")}
			}
		}
	}
}

fn on_bid(setback_game: Rc<RefCell<SetbackGame>>, bid: i32) {todo!()}
fn on_play_card(setback_game: Rc<RefCell<SetbackGame>>, card: Card) {todo!()}

fn on_send_create_game(setback_game: Rc<RefCell<SetbackGame>>) {todo!()}
fn on_send_join(setback_game: Rc<RefCell<SetbackGame>>, code: slint::SharedString) {todo!()}
fn on_send_search_games(setback_game: Rc<RefCell<SetbackGame>>, filter: GameFilter) {todo!()}
fn on_send_game_meta(setback_game: Rc<RefCell<SetbackGame>>) {todo!()}//TODO find out how to pass GameMeta
fn on_send_player(setback_game: Rc<RefCell<SetbackGame>>, player: Player) {todo!()}
fn on_send_players(setback_game: Rc<RefCell<SetbackGame>>, players: ModelRc<Player>) {todo!()}
fn on_send_ready(setback_game: Rc<RefCell<SetbackGame>>, ready: bool) {todo!()}

fn on_logout(setback_game: Rc<RefCell<SetbackGame>>) {todo!()}
fn on_get_account(setback_game: Rc<RefCell<SetbackGame>>) {todo!()}

fn on_fetch_players(setback_game: Rc<RefCell<SetbackGame>>) -> ModelRc<Player> {
	ModelRc::from(
		setback_game.borrow().fetch_players()
			// .as_slice()
			.iter()
			.map(|player| {
				Player {
					local: false,
					player_type: match player.player_data.player_type {
						shared::PlayerType::AI => PlayerType::AI,
						_ => PlayerType::Human
					},
					team: player.player_data.team_id as i32,
					username: SharedString::from(&player.player_data.username),
				}
			})
			.collect::<Vec<Player>>()
			.as_slice()
	)
}
fn on_fetch_games(setback_game: Rc<RefCell<SetbackGame>>) {todo!()}

fn on_calc_player_pos(setback_game: Rc<RefCell<SetbackGame>>, index: i32) -> (f32, f32) {
	let theta: f32 = 2. * PI * index as f32 / setback_game.borrow().fetch_num_players() as f32 + 3. * PI / 2.;
	(theta.cos() * ELLIPSE_RADIUS, ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS)
}

//Multiplayer game loop callback functions

fn ssp_callback(game: Rc<RefCell<SetbackGame>>) -> fn(message: Result<shared::Ssp, shared::ClientError>) {
	move |message: Result<shared::Ssp, shared::ClientError>| {
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