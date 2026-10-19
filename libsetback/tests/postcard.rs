#[cfg(test)]
mod postcard_size_tests {
	use {
		arrayvec::ArrayVec,
		setback::*,
	};
	
	/// Every encoded event (including the timestamp wrapper) must stay below this.
	const MAX_ENCODED: usize = 256;
	
	fn max_card() -> Card {
		Card {
			number: u8::MAX,
			suit: Suit::Spades
		}
	}
	fn max_bid() -> Bid {
		Bid::Override
	}
	fn max_prompt_error() -> PromptError {
		NetError::ServerError.into()
	}
	
	fn max_round_scores() -> RoundScores {
		RoundScores {
			high_winner_team_id: u8::MAX,
			low_winner_team_id: u8::MAX,
			jack_winner_team_id: Some(u8::MAX),
			game_winner_team_id: Some(u8::MAX),
			round_team_game_scores: ArrayVec::from([u8::MAX; MAX_PLAYERS]),
			total_team_scores: ArrayVec::from([i8::MIN; MAX_PLAYERS]),
		}
	}
	
	fn largest_events() -> Vec<(&'static str, GameEvent)> {
		let full_hand: ArrayVec<Card, CARDS_PER_PLAYER> =
			std::iter::repeat_with(max_card).take(CARDS_PER_PLAYER).collect();
		
		vec![
			("Init", GameEvent::Init { idx: u8::MAX }),
			("PlayerBid", GameEvent::PlayerBid { idx: u8::MAX, bid: max_bid() }),
			("PlayerPlayedCard", GameEvent::PlayerPlayedCard { idx: u8::MAX, card: max_card() }),
			("PromptBid", GameEvent::PromptBid(Some(max_prompt_error()))),
			("PromptCard", GameEvent::PromptCard(Some(max_prompt_error()))),
			("DealCards", GameEvent::DealCards { hand: full_hand, idx: u8::MAX }),
			("RoundScores", GameEvent::RoundScores(max_round_scores())),
			("GameOver", GameEvent::GameOver),
		]
	}
	
	/// Fails to compile when a variant is added to GameEvent, as a reminder
	/// to add its largest form to `largest_events` above.
	#[allow(dead_code)]
	fn assert_all_variants_covered(e: &GameEvent) {
		match e {
			GameEvent::Init { .. }
			| GameEvent::PlayerBid { .. }
			| GameEvent::PlayerPlayedCard { .. }
			| GameEvent::PromptBid(_)
			| GameEvent::PromptCard(_)
			| GameEvent::DealCards { .. }
			| GameEvent::RoundScores(_)
			| GameEvent::GameOver => {}
		}
	}
	
	#[test]
	fn largest_encoded_events_fit() {
		let mut largest = 0;
		
		for (name, event) in largest_events() {
			let buf = Vec::<u8>::with_capacity(u8::MAX as usize);
			let logged = LoggedGameEventRef { timestamp_ms: i64::MAX, event: &event };
			let bytes = postcard::to_extend(&logged, buf).unwrap();
			
			println!("{name}: {} bytes", bytes.len());
			assert!(
				bytes.len() < MAX_ENCODED,
				"{name} encodes to {} bytes, which is not under {MAX_ENCODED}",
				bytes.len()
			);
			largest = largest.max(bytes.len());
		}
		
		println!("largest encoded event: {largest} bytes");
	}
}