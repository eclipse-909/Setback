use {
	resources::client_items::*,
	std::sync::Arc,
	tokio::{
		runtime::{Runtime, Builder},
		sync::{oneshot, mpsc, RwLock, RwLockWriteGuard},
		task::JoinHandle
	}
};

///Opaque pointer for external code. Holds game state and used in functions which manage game flow.
pub struct Game {
	runtime: Runtime,
	game_loop: Option<JoinHandle<()>>,
	terminator: Option<oneshot::Sender<()>>,
	bid_tx: Option<mpsc::Sender<()>>,
	play_card_tx: Option<mpsc::Sender<()>>,
	sp_game: Arc<RwLock<SPGame>>
}

fn new_game() -> Option<Game> {
	let Ok(runtime) = Builder::new_multi_thread().enable_all().build() else {return None;};
	let Some(sp_game) = SPGame::default() else {return None;};
	Game{runtime, game_loop: None, terminator: None, bid_tx: None, play_card_tx: None, sp_game: Arc::new(RwLock::new(sp_game))}.into()
}

///Sends the kill-signal to end the game loop.
fn terminate_game(game: &mut Game) {
	if let Some(terminator) = game.terminator.take() {
		let _ = terminator.send(());
	}
}

///Blocks the current thread until the game loop ends
fn block_on_game_loop(game: &mut Game) {
	if let Some(handle) = game.game_loop.take() {
		game.runtime.block_on(async move {let _ = handle.await;});
	}
}

///De-allocates the Game object. Blocks until the game loop is finished, which shouldn't take long.
///Returns immediately if game is null. Do not call this multiple times, and do not use the pointer afterward.
fn drop_game(game: &mut Game) {
	if let Some(terminator) = game.terminator.take() {
		let _ = terminator.send(());
	}
	block_on_game_loop(game);
}

///Adds a player to the game and initializes its data. Blocks until sp_game has write access.
///Returns if successful. False should be treated as an unrecoverable error.
fn add_player(game: &mut Game, human: bool, team_id: u8) -> bool {
	let Some(player) = (if human {Player::new_human(team_id)} else {Player::new_ai(team_id)}) else {return false;};
	game.runtime.block_on(async move {game.sp_game.write().await.players.push(player)});
	true
}

///Clears the list of players. This should only be called in the setup and not in-game.
///This should be used whenever a change is made to the order of players. The intention is
///to be lazy and clear the list and call add_player again for each player. This means that
///the caller needs to have a list of team_ids and human booleans to represent the players.
///This blocks until sp_game has write access.
fn clear_players(game: &mut Game) {
	game.runtime.block_on(async move {game.sp_game.write().await.players.clear();});
}

///Initializes any remaining data and starts the game loop.
///You should not call [add_player] or [clear_players] after this has been called.
///Returns whether it is successful. If it returns false, the caller should treat
///it as an error. If it returns true, all other communication should be handled in the callbacks.
///
///[bid_callback] Notifies you what an AI has bid. It will already be the next player's turn when you receive this.
///[play_card_callback] Notifies you what card an AI has played. It will already be the next player's turn when you receive this.
///[my_turn_callback] Notifies you when it is your turn, and whether you are bidding or playing a card this turn.
///[scores_callback] Notifies you when the round has ended and what the scores of the round are. It will already be the next player's turn when you receive this.
///[error_callback] Notifies you if there's an unrecoverable error during the game loop.
fn start_game(
	game: &mut Game,
	bid_callback: fn(index: u8, bid: u8),
	play_card_callback: fn(index: u8, card: Card),
	my_turn_callback: fn(game: &mut Game, bidding: bool),
	scores_callback: fn(scores: RoundScores),
	game_over_callback: fn(),
	error_callback: fn(error: ServerError)
) -> bool {
	let (terminator, terminator_rx) = oneshot::channel::<()>();
	let (bid_tx, mut bid_rx) = mpsc::channel::<()>(1);
	let (play_card_tx, mut play_card_rx) = mpsc::channel::<()>(1);
	game.terminator = Some(terminator);
	game.bid_tx = Some(bid_tx);
	game.play_card_tx = Some(play_card_tx);
	let started: bool = game.runtime.block_on(async move {
		let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = game.sp_game.write().await;
		let num_players: usize = sp_game_write_guard.players.len();
		if !sp_game_write_guard.start_game() {return false;}
		sp_game_write_guard.deal_cards().is_ok()
	});
	if !started {return false;}
	let sp_game: Arc<RwLock<SPGame>> = game.sp_game.clone();
	game.game_loop = Some(game.runtime.spawn(async move {
		tokio::select! {
			_ = async move {
				let mut turn_result: Result<TurnResult, ServerError> = Ok(TurnResult::IsAI(sp_game.read().await.is_ai()));
				loop {
					match turn_result {
						Ok(TurnResult::IsAI(true)) => {
							let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = sp_game.write().await;
							if sp_game_write_guard.game_data.turn_number == 0 {
								let index: u8 = sp_game_write_guard.game_data.turn_index;
								let ai_bid: u8 = sp_game_write_guard.ai_bid();
								turn_result = sp_game_write_guard.next_turn();
								bid_callback(index, ai_bid);
							} else {
								let Some(card) = sp_game_write_guard.ai_play_card() else {
									return error_callback(ServerError::Indeterminate);
								};
								let index: u8 = sp_game_write_guard.game_data.turn_index;
								turn_result = sp_game_write_guard.next_turn();
								play_card_callback(index, card);
							}
						}
						Ok(TurnResult::IsAI(false)) => {
							let is_bidding: bool = sp_game.read().await.game_data.turn_number == 0;
							my_turn_callback(game, is_bidding);//TODO I may need to wrap game in an Arc RwLock because I can't pass a mutable reference between threads
							let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = sp_game.write().await;
							if sp_game_write_guard.game_data.turn_number == 0 {
								if bid_rx.recv().await.is_none() {
									return error_callback(ServerError::Indeterminate);
								}
							} else if play_card_rx.recv().await.is_none() {
								return error_callback(ServerError::Indeterminate);
							}
							turn_result = sp_game_write_guard.next_turn();
						}
						Ok(TurnResult::ReDeal) => {
							let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = sp_game.write().await;
							if let Err(error) = sp_game_write_guard.deal_cards() {
								return error_callback(error);
							}
							turn_result = Ok(TurnResult::IsAI(sp_game_write_guard.is_ai()));
						}
						Ok(TurnResult::EndRound(scores)) => {
							let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = sp_game.write().await;
							if let Err(error) = sp_game_write_guard.deal_cards() {
								return error_callback(error);
							}
							turn_result = Ok(TurnResult::IsAI(sp_game_write_guard.is_ai()));
							scores_callback(scores);
						}
						Ok(TurnResult::GameOver(scores)) => {
							scores_callback(scores);
							return game_over_callback();
						}
						Err(error) => {
							return error_callback(error);
						}
					}
				}
			} => {}
			_ = terminator_rx => {}
		}
	}));
	true
}

///Make a bid. If the game loop is waiting on the user input bid, this will allow it to continue.
///Blocks until sp_game has write access.
fn make_bid(game: &mut Game, bid: u8) -> Result<bool, ServerError> {
	game.runtime.block_on(async move {
		let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = game.sp_game.write().await;
		if !sp_game_write_guard.validate_bid(bid) {return Ok(false);}
		let Some(tx) = &game.bid_tx else {return Err(ServerError::Indeterminate);};
		if tx.try_send(()).is_err() {return Err(ServerError::Indeterminate);}
		Ok(true)
	})
}

///Play a card. If the game loop is waiting on the user input play, this will allow it to continue.
fn play_card(game: &mut Game, card: Card) -> Result<bool, ServerError> {
	game.runtime.block_on(async move {
		let mut sp_game_write_guard: RwLockWriteGuard<SPGame> = game.sp_game.write().await;
		match sp_game_write_guard.validate_card(card) {
			Ok(true) => {}
			Ok(false) => {return Ok(false);}
			Err(error) => {return Err(error);}
		}
		let Some(tx) = &game.play_card_tx else {return Err(ServerError::Indeterminate);};
		if tx.try_send(()).is_err() {return Err(ServerError::Indeterminate);}
		Ok(true)
	})
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		std::io::{Write, stderr, stdin},
		tokio::sync::RwLockReadGuard
	};
	
	#[test]
	fn test_ai_game() {
		let mut game: Game = new_game().unwrap();
		for i in 0..8 {
			assert!(add_player(&mut game, false, i % 2));
		}
		assert!(start_game(&mut game, signal_ai_bid, signal_ai_play_card, signal_my_turn, signal_round_scores, signal_game_over, signal_game_loop_error));
		block_on_game_loop(&mut game);
		drop_game(&mut game);
	}
	
	fn signal_ai_bid(index: u8, bid: u8) {eprintln!("Player {} bid {}", index, bid);}
	fn signal_ai_play_card(index: u8, card: Card) {eprintln!("Player {} played {}", index, card);}
	///If not bidding, this blocks until sp_game has read access.
	fn signal_my_turn(game: &mut Game, bidding: bool) {
		if bidding {
			loop {
				eprint!("Please enter your bid: ");
				let _ = stderr().flush();
				let mut input: String = String::new();
				if stdin().read_line(&mut input).is_err() {continue;}
				let Ok(n) = input.trim().parse::<u8>() else {eprint!("\nInvalid input. "); continue;};
				match make_bid(game, n) {
					Ok(true) => {eprintln!(); return;}
					Ok(false) => {eprint!("\nInvalid bid. ");}
					Err(_) => {terminate_game(game); return;}
				}
			}
		} else {
			game.runtime.block_on(async move {
				let sp_game_read_guard: RwLockReadGuard<SPGame> = game.sp_game.read().await;
				loop {
					eprintln!("Your cards:");
					for (index, card) in sp_game_read_guard.players[sp_game_read_guard.game_data.turn_index].hand.iter().enumerate() {
						eprintln!("{}: {}", index, card);
					}
					eprint!("Please enter the index of the card you want to play: ");
					let _ = stderr().flush();
					let mut input: String = String::new();
					if stdin().read_line(&mut input).is_err() {continue;}
					let Ok(n) = input.trim().parse::<u8>() else {eprint!("\nInvalid input. "); continue;};
					if n > sp_game_read_guard.players[sp_game_read_guard.game_data.turn_index].hand.len() {eprint!("\nInvalid input. "); continue;}
					match play_card(game, sp_game_read_guard.players[sp_game_read_guard.game_data.turn_index].hand[n].clone()) {
						Ok(true) => {eprintln!(); return;}
						Ok(false) => {eprint!("\nInvalid card index. ");}
						Err(_) => {terminate_game(game); return;}
					}
				}
			});
		}
	}
	fn signal_round_scores(scores: RoundScores) {eprintln!("Scores {:#?}", scores);}
	fn signal_game_over() {eprintln!("Game Over");}
	fn signal_game_loop_error(error: ServerError) {eprintln!("Server Error: {}", error);}
}