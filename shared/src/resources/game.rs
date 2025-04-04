use {
	super::{
		game_items::{Card, Suit, Player, PlayerType, RoundScores, Team, J, Q, K, A, CARDS_PER_PLAYER},
		util::{MAX_PLAYERS, Error},
		setup_data::SetupData
	},
	bincode::{Encode, Decode},
	std::collections::HashMap,
	rand::{
		rngs::ThreadRng,
		Rng, rng
	}
};

/// Game Data to manage the game state after it starts.
#[derive(Encode, Decode)]
pub struct Game {
	pub setup_data: SetupData,
	pub teams: Vec<Team>,
	pub deck: Vec<Card>,
	pub pile: Vec<Card>,
	pub trump: Suit,
	pub lead: Suit,
	///The index of the player who owns the pile. Played the highest_card.
	pub pile_owner: u8,
	///the card that was played to take ownership of the pile. Played by the pile_owner
	pub highest_card: Card,
	///index of the player who won the bid. Made the highes_bid.
	pub highest_bidder: u8,
	///the highest bid so far
	pub highest_bid: u8,
	///the index of the player whose turn it is
	pub turn_index: u8,
	///the turn number in the round (0..=6 where turn 0 is the bidding turn)
	pub turn_number: u8,
	///the index of the player who led this turn
	pub lead_index: u8,
	pub dealer_index: u8,
	pub round_number: u8
}

impl Game {
	/// Starts the game and initializes the game data.
	/// The caller must deal the cards after this function has been called.
	#[inline(always)]
	pub fn new(mut setup_data: SetupData) -> Self {
		//consolidate teams so the team_ids match indices contiguously starting at 0
		let mut team_ids: HashMap<u8, u8> = HashMap::with_capacity(setup_data.players.len());
		for player in setup_data.players.iter() {
			team_ids.entry(player.team_id).or_insert(0);
		}
		//create team objects in the game data
		let mut teams: Vec<Team> = Vec::with_capacity(team_ids.len());
		for _ in 0..team_ids.len() {
			teams.push(Team::default());
		}
		let mut pairs: Vec<(&u8, &mut u8)> = team_ids.iter_mut().collect::<Vec<(&u8, &mut u8)>>();
		pairs.sort_by(|item1, item2| {item1.0.cmp(item2.0)});
		for (index, (_, new_id)) in pairs.iter_mut().enumerate() {
			**new_id = index as u8;
		}
		for player in setup_data.players.iter_mut() {
			let Some(new_id) = team_ids.get(&player.team_id) else {continue;};
			player.team_id = *new_id;
		}
		//create deck of 52
		let mut deck: Vec<Card> = Vec::with_capacity(52);
		for number in 2..=A {
			for suit in Suit::iter() {
				deck.push(Card {suit, number});
			}
		}
		shuffle(&mut deck);

		Self {
			setup_data,
			teams,
			deck,
			pile: Vec::with_capacity(MAX_PLAYERS as usize * CARDS_PER_PLAYER as usize),
			trump: Suit::Hearts,
			lead: Suit::Hearts,
			pile_owner: 0,
			highest_card: Card{suit: Suit::Hearts, number: 0},
			highest_bidder: 0,
			highest_bid: 0,
			turn_index: 0,
			turn_number: 0,
			lead_index: 0,
			dealer_index: 0,
			round_number: 0
		}
	}

	///Jumps to the next player's turn and increments turn and round data.
	///If Ok, the TurnResult represents what action the caller needs to take immediately after.
	///If Err, the caller needs to handle it, usually be terminating the game.
	pub fn next_turn(&mut self) -> Result<TurnResult, Error> {
		if self.turn_number == 1 && self.pile.is_empty() {
			self.turn_index = self.highest_bidder;
			return Ok(TurnResult::NextTurn);
		}
		if self.turn_number == 0 && self.turn_index == self.dealer_index {
			return if self.highest_bid == 0 {
				//re-deal
				Ok(TurnResult::ReDeal)
			} else {
				//carry on to the first turn
				self.turn_number = 1;
				self.turn_index = self.highest_bidder;
				Ok(TurnResult::NextTurn)
			}
		}
		self.turn_index += 1;
		if self.turn_index >= self.setup_data.players.len() as u8 {
			//loop the iteration
			self.turn_index = 0;
		}
		if self.pile.len() == self.setup_data.players.len() {
			//when the turn is over
			//give pile to owning team
			self.teams[self.setup_data.players[self.pile_owner as usize].team_id as usize].pile.append(&mut self.pile);
			self.turn_index = self.pile_owner;
			self.turn_number += 1;
			if self.turn_number == 7 {
				//if the round has ended
				let scores: RoundScores = self.end_round()?;
				//see if someone won
				let mut highest_score: i8 = 0;
				let mut second_highest_score: i8 = 0;
				for team in self.teams.iter() {
					if team.score >= highest_score {
						second_highest_score = highest_score;
						highest_score = team.score;
					}
				}
				if highest_score >= self.setup_data.max_score && (highest_score > second_highest_score + 1 || (!self.setup_data.win_by_2 && highest_score > second_highest_score)) {
					return Ok(TurnResult::GameOver(scores, self.teams.iter().map(|team| {team.score}).collect()));
				}
				return Ok(TurnResult::EndRound(scores));
			}
		}
		Ok(TurnResult::NextTurn)
	}
	
	///Caller must deal cards for next round after this function is called.
	///Use RoundScores to notify players the results of the round.
	pub fn end_round(&mut self) -> Result<RoundScores, Error> {
		//calculate scores
		let mut highest: u8 = 2;
		let mut highest_index: u8 = 0;
		let mut lowest: u8 = A;
		let mut lowest_index: u8 = 0;
		let mut jack_index: Option<u8> = None;
		let mut game: u8 = 0;
		let mut game_runner_up: u8 = 0;
		let mut game_index: Option<u8> = None;
		let mut team_game_scores: Vec<u8> = Vec::with_capacity(MAX_PLAYERS as usize);
		for (team_index, team) in self.teams.iter().enumerate() {
			let (h, l, j, g) = calc_points(&team.pile, self.trump);
			if h > highest {
				highest = h;
				highest_index = team_index as u8;
			}
			if l < lowest {
				lowest = l;
				lowest_index = team_index as u8;
			}
			if j {jack_index = Some(team_index as u8);}
			if g >= game {
				game_runner_up = game;
				game = g;
				game_index = Some(team_index as u8);
			}
			team_game_scores.push(g);
		}
		if game == game_runner_up {
			game_index = None;
		}
		let mut round_scores: HashMap<u8, i8> = HashMap::with_capacity(4);
		round_scores.entry(highest_index).and_modify(|score| {*score += 1;}).or_insert(1);
		round_scores.entry(lowest_index).and_modify(|score| {*score += 1;}).or_insert(1);
		if let Some(index) = jack_index {
			round_scores.entry(index).and_modify(|score| {*score += 1;}).or_insert(1);
		}
		if let Some(index) = game_index {
			round_scores.entry(index).and_modify(|score| {*score += 1;}).or_insert(1);
		}
		round_scores.entry(self.setup_data.players[self.highest_bidder as usize].team_id).and_modify(|bidder_score| {
			if *bidder_score < self.highest_bid as i8 {
				*bidder_score = -(self.highest_bid as i8);
			}
		}).or_insert(-(self.highest_bid as i8));
		for (team_id, score) in round_scores {
			self.teams[team_id as usize].score += score;
		}
		let mut team_scores: Vec<i8> = Vec::with_capacity(self.teams.len());
		for team in self.teams.iter() {
			team_scores.push(team.score);
		}
		self.dealer_index += 1;
		if self.dealer_index >= self.setup_data.players.len() as u8 {
			self.dealer_index = 0;
		}
		self.round_number += 1;
		Ok(RoundScores {
			high_team_id: highest_index,
			low_team_id: lowest_index,
			jack_team_id: jack_index,
			game_team_id: game_index,
			team_game_scores,
			team_scores
		})
	}
	
	pub fn deal_cards(&mut self) {
		//reset game data
		self.highest_bid = 0;
		self.turn_number = 0;
		self.turn_index = self.dealer_index + 1;
		if self.turn_index >= self.setup_data.players.len() as u8 {
			self.turn_index = 0;
		}
		//collect cards and shuffle
		self.deck.append(&mut self.pile);
		for player in self.setup_data.players.iter_mut() {
			self.deck.append(&mut player.hand);
		}
		for team in self.teams.iter_mut() {
			self.deck.append(&mut team.pile);
		}
		shuffle(&mut self.deck);
		//deal cards
		const ROUNDS: u8 = 2;
		for _ in 0..ROUNDS {
			for player in self.setup_data.players.iter_mut() {
				for card in self.deck.drain(0..(CARDS_PER_PLAYER / ROUNDS) as usize) {
					player.hand.push(card);
				}
			}
		}
	}
	
	///Which ever player's turn it currently is, this checks if that player is an AI.
	pub fn is_ai_turn(&self) -> bool {matches!(self.setup_data.players[self.turn_index as usize].player_type, PlayerType::AI)}
	
	///Validates the bid and updates game state. Returns early if the bid was invalid. Call [Self::next_turn] after this.
	pub fn validate_bid(&mut self, bid: u8) -> bool {
		if bid == 0 {return true;}
		if bid == 1 || bid > 4 || bid <= self.highest_bid {return false;}
		self.highest_bidder = self.turn_index;
		self.highest_bid = bid;
		if bid == 4 {
			self.turn_number = 1;
		}
		true
	}
	
	///Returns early if the card played is invalid. Call [Self::next_turn] after this.
	pub fn validate_card(&mut self, card: Card) -> bool {
		let player: &mut Player = &mut self.setup_data.players[self.turn_index as usize];
		let Some(card_index) = player.hand.iter().position(|c| {*c == card}) else {return false;};
		//update game state
		if self.pile.is_empty() {
			//the first card played of the turn
			self.pile_owner = self.turn_index;
			self.lead_index = self.turn_index;
			self.highest_card = card;
			self.lead = card.suit;
			if self.turn_number == 1 {
				//first card played of the round
				self.trump = card.suit;
			}
		} else {
			//verify legality of card
			if card.suit != self.trump && card.suit != self.lead && player.can_follow(self.lead) {return false;}
			//calculate if new owner and update game data
			if (card.suit == self.highest_card.suit && card.number > self.highest_card.number) || (self.highest_card.suit != self.trump && card.suit == self.trump) {
				self.highest_card = card;
				self.pile_owner = self.turn_index;
			}
		}
		//put card on pile
		self.pile.push(player.hand.remove(card_index));
		true
	}
	
	///Algorithm for deciding how the AI should bid
	fn ai_eval_bid(&self) -> (Suit, u8) {
		let player: &Player = &self.setup_data.players[self.turn_index as usize];
		let mut suit_points: HashMap<Suit, u8> = HashMap::with_capacity(4);
		for suit in Suit::iter() {
			suit_points.insert(suit, 0);
		}
		for card in player.hand.iter() {
			let Some(points) = suit_points.get_mut(&card.suit) else {continue;};
			*points += match card.number {
				2 => {10}
				3 => {if self.setup_data.players.len() == MAX_PLAYERS as usize {4} else {8}}
				4 => {3}
				5 => {4}
				6 => {5}
				7 => {10}
				8=> {12}
				9 => {20}//9 through A can't add up to more than 255
				10 => {25}
				J => {40}
				Q => {35}
				K => {45}
				A => {90}//A should be more than any 2 other things combined
				_ => {0}
			}
		}
		let (suit, num) = suit_points.iter().max_by(|(_, num1), (_, num2)| {num1.cmp(num2)}).unwrap();
		(*suit, *num)
	}
	
	///Uses a card-scoring system, the turn order, and the game score to determine the bid
	pub fn ai_bid(&mut self) -> u8 {
		let (_, points) = self.ai_eval_bid();
		let mut bid: u8 = match points {
			..=44 => {0}//pass
			45..=99 => {1}//minimum play out of desperation
			100..=139 => {2}//A + one more good card
			140..=179 => {3}//A + face card or two + maybe one more
			180.. => {4}//A + K + J + one more good card
		};
		//if you are one of the first to bid, maybe bid higher either to force an opponent to outbid when they shouldn't
		//or to prevent opponents from bidding to try to be the highest bidder
		if bid != 0 && bid != 4 && ((90..=99).contains(&bid) || (125..=139).contains(&bid)) {
			for i in 0..self.setup_data.players.len() - 5 {
				let mut index: u8 = self.turn_index + i as u8;
				if index >= self.setup_data.players.len() as u8 {index -= self.setup_data.players.len() as u8;}
				if index == self.turn_index {
					bid += 1;
					break;
				}
			}
		}
		//if opponents are about to win, make risky bids
		if let Some((index, lead_team)) = self.teams.iter().enumerate().max_by(|(_, team1), (_, team2)| {team1.score.cmp(&team2.score)}) {
			let player: &Player = &self.setup_data.players[self.turn_index as usize];
			if player.team_id != index as u8 && lead_team.score >= self.setup_data.max_score - 4 && self.teams[player.team_id as usize].score < self.setup_data.max_score - 4 && bid != 4 {
				bid += 1;
			}
		}
		if bid <= 1 || bid <= self.highest_bid {bid = 0;}
		bid
	}
	
	///Plays the highest card if this player had the highest bid.
	///Currently picks a random valid card until a proper AI is implemented.
	pub fn ai_play_card(&mut self) -> Card {
		let player: &Player = &self.setup_data.players[self.turn_index as usize];
		//Highest bidder starts the round, and the leading card is a unique calculation
		if self.turn_number == 1 && self.pile.is_empty() {
			let (suit, _) = self.ai_eval_bid();//Suit of highest bid should not have changed, so we will just recalculate it instead of storing it
			let highest: Card = *player.hand.iter().filter(|card| {card.suit == suit}).max_by(|card1, card2| {card1.number.cmp(&card2.number)}).unwrap();
			return highest;
		}
		let playable_card: Vec<Card> = if !self.pile.is_empty() && player.can_follow(self.lead) {
			player.hand.iter().filter(|card| {card.suit == self.lead || card.suit == self.trump}).cloned().collect()
		} else {
			player.hand.clone()
		};
		
		//For now I'm gonna get a random card
		let mut rng: ThreadRng = rng();
		let index: usize = rng.random_range(0..playable_card.len());
		
		
		//TODO make AI to choose card to play
		
		/*
		Factors:
			legal cards in hand
			number of players
			number of teams
			turn index between leader and last to play
			team number of players
			team that owns the pile
			other teams that could take the pile
			cards that have been played, particularly trump cards
			the highest trump card that hasn't been played
			who's following trump suit
			if the pile contains A, J, and/or 2 (or 3 if 2 hasn't been played yet)
			how many game points are in the pile
			
		Heuristics:
			if AI can follow, only allow follow suit or trump suit, else allow all cards
		*/
		
		let card: Card = playable_card[index];
		card
	}
}

fn shuffle(deck: &mut [Card]) {
	let mut rng: ThreadRng = rng();
	for _ in 0..7 {
		for i in (1..deck.len()).rev() {
			let j: usize = rng.random_range(0..=i);
			deck.swap(i, j);
		}
	}
}

///returns (high, low, jack, game)
#[inline(always)]
fn calc_points(pile: &[Card], trump: Suit) -> (u8, u8, bool, u8) {
	let mut highest: u8 = 2;
	let mut lowest: u8 = A;
	let mut jack: bool = false;
	let mut game: u8 = 0;
	for card in pile.iter() {
		match card.number {
			10 => {game += 10;}
			J => {
				game += 1;
				if card.suit == trump {jack = true;}
			}
			Q => {game += 2;}
			K => {game += 3;}
			A => {game += 4}
			_ => {}
		}
		if card.suit != trump {continue;}
		if card.number > highest {
			highest = card.number;
		}
		if card.number < lowest {
			lowest = card.number;
		}
	}
	(highest, lowest, jack, game)
}

#[repr(u8)]
pub enum TurnResult {
	NextTurn,
	///A re-deal is required after this one
	ReDeal,
	///A re-deal is required for this one too
	EndRound(RoundScores),
	///Round scores and team scores for whole game
	GameOver(RoundScores, Vec<i8>)
}