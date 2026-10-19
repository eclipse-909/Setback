#[cfg(test)]
mod symmetric_serde {
	use {
		std::assert_matches,
		setback::*,
	};
	
	#[test]
	fn ref_and_owned_game_event_are_compatible() {
		let ev = GameEvent::PlayerPlayedCard {card: Card {suit: Suit::Spades, number: 9}, idx: 3};
		let mut buf = [0u8; u8::MAX as usize];
		let ts = 42;
		let bytes = postcard::to_slice(&LoggedGameEventRef { event: &ev, timestamp_ms: ts }, &mut buf).unwrap();
		let back: LoggedGameEvent = postcard::from_bytes(bytes).unwrap();
		assert_eq!(back.timestamp_ms, ts);
		assert_matches!(back.event, ev);
	}
	
	fn player_type_serde() {
		// TODO:
		// Create a PlayerType::HumanConnection,
		// serialize it,
		// deserialize it,
		// and confirm that it's a PlayerType::HumanPlayer
		// and that all the data is the same.
	}
}