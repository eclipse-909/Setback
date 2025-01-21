#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::SharedString;
use {
	shared::{
		game::GameBuilder,
		client::GameClient,
		RoundScores, ServerError
	},
	std::f32::consts::PI,
	tokio::{
		task::JoinHandle,
		sync::mpsc
	},
	tokio_util::sync::CancellationToken,
	slint::ModelRc
};

slint::include_modules!();

macro_rules! set_callbacks {
    ($main_window:ident, $setback_game:ident, {$($set_callback:stmt),* $(,)?}) => {
	    $(set_callbacks!($set_callback)),*
    };
	($callback:ident) => {
		//no args
		main_window.global::<GameState>().$callback($callback);
	};
	($callback:ident < $game:ident) => {
		//capture game
		main_window.global::<GameState>().$callback(||$callback(&mut $game));
	};
	($callback:ident < ($($arg:ident),* $(,)?)) => {
		//args
		main_window.global::<GameState>().$callback(|$($arg),* |$callback($($arg),* ));
	};
	($callback:ident < $game:ident < ($($arg:ident),* $(,)?)) => {
		//capture game and args
		main_window.global::<GameState>().$callback(|$($arg),* |$callback(&mut $game, $($arg),* ));
	};
}

const ELLIPSE_RADIUS: f32 = 525.;
const ELLIPSE_HEIGHT_RATIO: f32 = 0.625;

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
	let mut setback_game: SetbackGame = SetbackGame::default();
	let main_window: MainWindow = MainWindow::new()?;
	set_callbacks!(main_window, setback_game, {
		on_exit,
		on_bid < setback_game < (bid),
		on_play_card < setback_game,
		on_send_create_game < setback_game,
		on_send_join < setback_game < (code),
		on_send_search_games < setback_game < (filter),
		on_send_game_meta < setback_game,
		on_send_player < setback_game < (player),
		on_send_players < setback_game < (players),
		on_send_ready < setback_game < (ready),
		on_logout < setback_game,
		on_get_account < setback_game,
		on_fetch_players < setback_game,
		on_fetch_games < setback_game,
		on_calc_player_pos < setback_game < (index)
	});
	main_window.run()
}

///Game state for the app
#[derive(Default)]
struct SetbackGame {
	scene_type: Scene,
	host_type: Host
}

impl SetbackGame {
	fn fetch_players(&self) -> Vec<shared::Player> {
		match &self.host_type {
			Host::None => {unreachable!()}
			Host::SingleplayerSetup(game_builder) => {todo!()}
			Host::SingleplayerPlaying(_, _) => {todo!()}
			Host::Host(game_client) => {todo!()}
			Host::Client(game_client) => {todo!()}
		}
	}
	fn fetch_num_players(&self) -> u8 {todo!()}
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

///Game state which stores the relation between the player and the type of game they're playing.
#[derive(Default)]
#[repr(u8)]
enum Host {
	///Not in a game.
	#[default]
	None,
	SingleplayerSetup(GameBuilder),
	SingleplayerPlaying(JoinHandle<()>, CancellationToken),
	///Created a multiplayer game as the host.
	Host(GameClient),
	///Joined a multiplayer game as a client.
	Client(GameClient)
}

fn on_exit() {std::process::exit(0);}

fn on_bid(setback_game: &mut SetbackGame, bid: i32) {todo!()}
fn on_play_card(setback_game: &mut SetbackGame, card: Card) {todo!()}

fn on_send_create_game(setback_game: &mut SetbackGame) {todo!()}
fn on_send_join(setback_game: &mut SetbackGame, code: slint::SharedString) {todo!()}
fn on_send_search_games(setback_game: &mut SetbackGame, filter: GameFilter) {todo!()}
fn on_send_game_meta(setback_game: &mut SetbackGame) {todo!()}//TODO find out how to pass GameMeta
fn on_send_player(setback_game: &mut SetbackGame, player: Player) {todo!()}
fn on_send_players(setback_game: &mut SetbackGame, players: ModelRc<Player>) {todo!()}
fn on_send_ready(setback_game: &mut SetbackGame, ready: bool) {todo!()}

fn on_logout(setback_game: &mut SetbackGame) {todo!()}
fn on_get_account(setback_game: &mut SetbackGame) {todo!()}

fn on_fetch_players(setback_game: &mut SetbackGame) -> ModelRc<Player> {
	ModelRc::from(
		setback_game.fetch_players()
			// .as_slice()
			.iter()
			.map(|player| {
				Player {
					local: false,
					player_type: match player.player_type {
						shared::PlayerType::Human(_) => {
							PlayerType::Human
						}
						shared::PlayerType::AI => {
							PlayerType::AI
						}
					},
					team: player.team_id as i32,
					username: SharedString::from(&player.username),
				}
			})
			.collect()
	)
}
fn on_fetch_games(setback_game: &mut SetbackGame) {todo!()}

fn on_calc_player_pos(setback_game: &mut SetbackGame, index: i32) -> (f32, f32) {
	let theta: f32 = 2. * PI * index as f32 / setback_game.fetch_num_players() as f32 + 3. * PI / 2.;
	(theta.cos() * ELLIPSE_RADIUS, ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS)
}

//Singleplayer game loop callback functions

// fn prompt_bid(tx: &app::Sender<GameEvent>) -> fn(&Vec<Card>, &mpsc::Sender<u8>) {
// 	|hand: &Vec<Card>, sender: &mpsc::Sender<u8>| {
// 		loop {
// 			println!("Cards in hand:");
// 			for (index, card) in hand.iter().enumerate() {
// 				println!("{}: {}", index, card);
// 			}
// 			print!("Please enter your bid: ");
// 			let _ = stdout().flush();
// 			let mut input: String = String::new();
// 			if stdin().read_line(&mut input).is_err() {continue;}
// 			let Ok(n) = input.trim().parse::<u8>() else {print!("\nInvalid input. "); continue;};
// 			sender.try_send(n).unwrap();
// 			return;
// 		}
// 	}
// }
// fn prompt_play_card(hand: &Vec<Card>, sender: &mpsc::Sender<Card>) {
// 	loop {
// 		println!("Cards in hand:");
// 		for (index, card) in hand.iter().enumerate() {
// 			println!("{}: {}", index, card);
// 		}
// 		print!("Please enter the index of the card you want to play: ");
// 		let _ = stdout().flush();
// 		let mut input: String = String::new();
// 		if stdin().read_line(&mut input).is_err() {continue;}
// 		let Ok(n) = input.trim().parse::<u8>() else {print!("\nInvalid input. "); continue;};
// 		if n > hand.len() as u8 {print!("\nInvalid input. "); continue;}
// 		sender.try_send(hand[n as usize].clone()).unwrap();
// 		return;
// 	}
// }
// fn signal_bid(index: u8, bid: u8) {println!("Player {} bid {}", index, bid);}
// fn signal_play_card(index: u8, card: Card) {println!("Player {} played {}", index, card);}
// fn signal_next_turn(index: u8, bidding: bool) {println!("Player {}'s turn to {}", index, if bidding {"bid"} else {"play a card."});}
// fn signal_round_scores(scores: RoundScores) {println!("Scores {:#?}", scores);}
// fn signal_game_over() {println!("Game Over");}
// fn signal_game_loop_error(error: ServerError) {println!("Server Error: {}", error);}