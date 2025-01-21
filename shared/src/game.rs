use {
	super::resources::{
		game::{Game, TurnResult},
		game_items::{Player, PlayerType, Card, RoundScores},
		util::ServerError
	},
	tokio::{
		sync::mpsc::Sender,
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken
};

///A struct to hold game state and communication with players.
///
///[GameBuilder::start_game] moves this into the function and starts the event loop.
pub struct GameBuilder {
	game: Game,
	senders: Vec<(Sender<u8>, Sender<Card>)>
}

impl Default for GameBuilder {
	fn default() -> Self {Self {game: Game::default(), senders: Vec::new()}}
}

impl GameBuilder {
	pub fn add_player(&mut self, human: bool, team_id: u8) {
		self.game.players.push(if human {Player::new_human(String::new(), team_id)} else {Player::new_ai(team_id)});
	}
	
	///Updates a player's status as AI or human.
	pub fn set_player(&mut self, index: u8, human: bool) {
		self.game.players[index as usize].player_type = if human {PlayerType::Human(None)} else {PlayerType::AI};
	}
	
	///Makes this player type a networked client.
	pub fn set_client(&mut self, index: u8) {
		todo!()
	}
	
	///Removes a player from the list, and reinserts it at the given index, sliding the remaining players if necessary.
	///Do not call this after starting the game.
	pub fn re_insert_player(&mut self, init_index: u8, new_index: u8) {
		let player: Player = self.game.players.remove(init_index as usize);
		self.game.players.insert(new_index as usize, player);
	}
	
	///Initializes any remaining data and starts the game loop.
	///If it returns Some, all other communication should be handled in the callbacks.
	///The return value is the join handle to the task the event loop is running on, so you can await it, and a cancellation token.
	///
	///[prompt_bid_callback] Blocking sync function where the player is given the bid sender to make a bid. Also informs them of the cards in their hand.
	///
	///[prompt_card_callback] Blocking sync function where the player is given the card sender to play a card. Also informs them of the cards in their hand.
	///
	///[bid_callback] Notifies you what an AI has bid. It will already be the next player's turn when you receive this.
	///
	///[play_card_callback] Notifies you what card an AI has played. It will already be the next player's turn when you receive this.
	///
	///[next_turn_callback] Notifies you whose turn it is and whether they're bidding.
	///
	///[scores_callback] Notifies you when the round has ended and what the scores of the round are. It will already be the next player's turn when you receive this.
	///
	///[game_over_callback] Notifies you when the game is over.
	///
	///[error_callback] Notifies you if there's an unrecoverable error during the game loop.
	pub fn start_game(
		mut self,
		prompt_bid_callback: fn(hand: &Vec<Card>, sender: &Sender<u8>),
		prompt_card_callback: fn(hand: &Vec<Card>, sender: &Sender<Card>),
		bid_callback: fn(index: u8, bid: u8),
		play_card_callback: fn(index: u8, card: Card),
		next_turn_callback: fn(index: u8, bidding: bool),
		scores_callback: fn(scores: RoundScores),
		game_over_callback: fn(),
		error_callback: fn(error: ServerError)
	) -> Option<(JoinHandle<()>, CancellationToken)> {
		if !self.game.start_game() {return None;}
		for player in self.game.players.iter_mut() {
			let Some(bid_tx) = player.bid_tx.take() else {return None;};
			let Some(card_tx) = player.card_tx.take() else {return None;};
			self.senders.push((bid_tx, card_tx));
		}
		if self.game.deal_cards().is_err() {return None;}
		let token: CancellationToken = CancellationToken::new();
		let cloned_token: CancellationToken = token.clone();
		Some((tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut turn_result: Result<TurnResult, ServerError> = Ok(TurnResult::NextTurn);
				next_turn_callback(self.game.game_data.turn_index, self.game.game_data.turn_number == 0);
				loop {
					match turn_result {
						Ok(TurnResult::NextTurn) => {
							let index: u8 = self.game.game_data.turn_index;
							let ai: bool = self.game.is_ai_turn();
							if self.game.game_data.turn_number == 0 {
								let bid: u8 = if ai {
									let b: u8 = self.game.ai_bid();
									assert!(self.game.validate_bid(b));
									b
								} else {
									prompt_bid_callback(&self.game.players[index as usize].hand, &self.senders[index as usize].0);
									let mut b: u8 = self.game.players[index as usize].bid_rx.as_mut().unwrap().recv().await.unwrap();
									while !self.game.validate_bid(b) {
										error_callback(ServerError::InvalidBid);
										prompt_bid_callback(&self.game.players[index as usize].hand, &self.senders[index as usize].0);
										b = self.game.players[index as usize].bid_rx.as_mut().unwrap().recv().await.unwrap();
									}
									b
								};
								turn_result = self.game.next_turn();
								bid_callback(index, bid);
							} else {
								if ai {
									self.senders[index as usize].1.send(self.game.ai_play_card()).await.unwrap();
								} else {
									prompt_card_callback(&self.game.players[index as usize].hand, &self.senders[index as usize].1);
								}
								let mut card: Card = self.game.players[index as usize].card_rx.as_mut().unwrap().recv().await.unwrap();
								while !self.game.validate_card(card.clone()) {
									error_callback(ServerError::InvalidCard);
									prompt_card_callback(&self.game.players[index as usize].hand, &self.senders[index as usize].1);
									card = self.game.players[index as usize].card_rx.as_mut().unwrap().recv().await.unwrap();
								}
								turn_result = self.game.next_turn();
								play_card_callback(index, card);
							}
							next_turn_callback(self.game.game_data.turn_index, self.game.game_data.turn_number == 0);
						}
						Ok(TurnResult::ReDeal) => {
							if let Err(error) = self.game.deal_cards() {
								return error_callback(error);
							}
							turn_result = Ok(TurnResult::NextTurn);
						}
						Ok(TurnResult::EndRound(scores)) => {
							if let Err(error) = self.game.deal_cards() {
								return error_callback(error);
							}
							turn_result = Ok(TurnResult::NextTurn);
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
			_ = cloned_token.cancelled() => {}
		}}), token))
	}
}