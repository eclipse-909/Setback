mod resources;

pub use resources::{
	connection::*,
	game_items::*,
	ssp::*,
	util::*
};

pub mod game {
	use {
		super::resources::{
			game::{Game, TurnResult},
			game_items::{Player, PlayerType, Card, GameEvent},
			util::Error,
			connection::ClientConnection
		},
		tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel, Sender},
		tokio_util::sync::CancellationToken
	};
	
	///A struct to hold game state and communication with players.
	///
	///[GameBuilder::start_game] moves this into the function and starts the event loop.
	#[derive(Default)]
	pub struct GameBuilder {
		pub game: Game //TODO I probably don't need to wrap Game with GameBuilder, but the intention is probably clearer this way
	}
	
	impl GameBuilder {
		pub fn add_ai(&mut self) {
			self.game.players.push(Player::new_ai(self.game.players.len() as u8));
		}
		
		///Updates a player's status as AI or human. This should only be called by the server.
		pub fn update_human(&mut self, index: u8, human: bool) {
			self.game.players[index as usize].player_data.player_type.set(if human {PlayerType::HumanEmpty} else {PlayerType::AI});
		}
	
		///Updates a player's team.
		pub fn update_team(&mut self, index: u8, team_id: u8) {
			self.game.players[index as usize].player_data.team_id = team_id;
		}
		
		///Makes this player type a networked client. This should only be called by the server.
		pub fn set_client(&mut self, index: u8, connection: ClientConnection) {
			self.game.players[index as usize].player_data.player_type.set(PlayerType::HumanConnection(connection));
		}
		
		///Removes a player from the list, and reinserts it at the given index, sliding the remaining players if necessary.
		///Do not call this after starting the game.
		pub fn re_insert_player(&mut self, init_index: u8, new_index: u8) {
			let player: Player = self.game.players.remove(init_index as usize);
			self.game.players.insert(new_index as usize, player);
		}
		
		///Initializes any remaining data and starts the game loop.
		///Further communication between the game and the players is handled through the returned [GameRuntime].
		pub fn start_game(mut self) -> GameRuntime {
			self.game.start_game();
			self.game.deal_cards();
			let token: CancellationToken = CancellationToken::new();
			let cloned_token: CancellationToken = token.clone();
			let (event_tx, event_rx) = unbounded_channel::<GameEvent>();
			let senders: Vec<(Sender<u8>, Sender<Card>)> = self.game.players
				.iter()
				.map(|player| {
					(player.bid_tx.clone(), player.card_tx.clone())
				})
				.collect();
			//spawn_local because self.game doesn't implement Send
			tokio::task::spawn_local(async move {tokio::select! {
				_ = async move {
					let mut turn_result: Result<TurnResult, Error> = Ok(TurnResult::NextTurn);
					loop {
						match turn_result {
							Ok(TurnResult::NextTurn) => {
								let index: u8 = self.game.game_data.turn_index;
								let ai: bool = self.game.is_ai_turn();
								let bidding: bool = self.game.game_data.turn_number == 0;
								event_tx.send(GameEvent::PlayerTurn{index}).unwrap();
								if bidding {
									let bid: u8 = if ai {
										let b: u8 = self.game.ai_bid();
										assert!(self.game.validate_bid(b));
										b
									} else {
										event_tx.send(GameEvent::PromptBid{index, hand: self.game.players[index as usize].player_data.hand.clone()}).unwrap();
										let mut b: u8 = self.game.players[index as usize].bid_rx.recv().await.unwrap();
										while !self.game.validate_bid(b) {
											event_tx.send(GameEvent::GameError(Error::InvalidBid)).unwrap();
											event_tx.send(GameEvent::PromptBid{index, hand: self.game.players[index as usize].player_data.hand.clone()}).unwrap();
											b = self.game.players[index as usize].bid_rx.recv().await.unwrap();
										}
										b
									};
									turn_result = self.game.next_turn();
									event_tx.send(GameEvent::PlayerBid {index, amount: bid}).unwrap();
								} else {
									let card: Card = if ai {
										let c: Card = self.game.ai_play_card();
										assert!(self.game.validate_card(c));
										c
									} else {
										event_tx.send(GameEvent::PromptCard{index, hand: self.game.players[index as usize].player_data.hand.clone()}).unwrap();
										let mut c: Card = self.game.players[index as usize].card_rx.recv().await.unwrap();
										while !self.game.validate_card(c) {
											event_tx.send(GameEvent::GameError(Error::InvalidCard)).unwrap();
											event_tx.send(GameEvent::PromptCard{index, hand: self.game.players[index as usize].player_data.hand.clone()}).unwrap();
											c = self.game.players[index as usize].card_rx.recv().await.unwrap();
										}
										c
									};
									turn_result = self.game.next_turn();
									event_tx.send(GameEvent::PlayerPlayedCard{index, card}).unwrap();
								}
							}
							Ok(TurnResult::ReDeal) => {
								self.game.deal_cards();
								event_tx.send(GameEvent::DealCards).unwrap();
								turn_result = Ok(TurnResult::NextTurn);
							}
							Ok(TurnResult::EndRound(scores)) => {
								self.game.deal_cards();
								turn_result = Ok(TurnResult::NextTurn);
								event_tx.send(GameEvent::RoundScores(scores)).unwrap();
							}
							Ok(TurnResult::GameOver(round_scores, game_scores)) => {
								event_tx.send(GameEvent::RoundScores(round_scores)).unwrap();
								return event_tx.send(GameEvent::GameOver(game_scores)).unwrap();
							}
							Err(error) => {
								return event_tx.send(GameEvent::GameError(error)).unwrap();
							}
						}
					}
				} => {}
				_ = cloned_token.cancelled() => {}
			}});
			GameRuntime {cancel_token: token, event_emitter: event_rx, prompt_senders: senders}
		}
	}
	
	///A struct to handle communication between the game and the players.
	///
	///[GameBuilder::start_game] moves the game into the function and starts the event loop.
	pub struct GameRuntime {
		pub cancel_token: CancellationToken,
		pub event_emitter: UnboundedReceiver<GameEvent>,
		pub prompt_senders: Vec<(Sender<u8>, Sender<Card>)>
	}
}