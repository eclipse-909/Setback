use {
	shared::{
		game::GameBuilder,
		Card, RoundScores, ServerError
	},
	std::io::{Write, stdout, stdin},
	tokio::sync::mpsc::Sender
};

///This is essentially an integration test
#[tokio::main]
async fn main() {
	let mut game: GameBuilder = GameBuilder::default();
	game.add_player(true, 0);
	for i in 1..8 {
		game.add_player(false, i % 2);
	}
	let (game_loop, _terminator) = game.start_game(
		prompt_bid,
		prompt_play_card,
		signal_bid,
		signal_play_card,
		signal_next_turn,
		signal_round_scores,
		signal_game_over,
		signal_game_loop_error
	).unwrap();
	game_loop.await.unwrap();
}
fn prompt_bid(hand: &Vec<Card>, sender: &Sender<u8>) {
	loop {
		println!("Cards in hand:");
		for (index, card) in hand.iter().enumerate() {
			println!("{}: {}", index, card);
		}
		print!("Please enter your bid: ");
		let _ = stdout().flush();
		let mut input: String = String::new();
		if stdin().read_line(&mut input).is_err() {continue;}
		let Ok(n) = input.trim().parse::<u8>() else {print!("\nInvalid input. "); continue;};
		sender.try_send(n).unwrap();
		return;
	}
}
fn prompt_play_card(hand: &Vec<Card>, sender: &Sender<Card>) {
	loop {
		println!("Cards in hand:");
		for (index, card) in hand.iter().enumerate() {
			println!("{}: {}", index, card);
		}
		print!("Please enter the index of the card you want to play: ");
		let _ = stdout().flush();
		let mut input: String = String::new();
		if stdin().read_line(&mut input).is_err() {continue;}
		let Ok(n) = input.trim().parse::<u8>() else {print!("\nInvalid input. "); continue;};
		if n > hand.len() as u8 {print!("\nInvalid input. "); continue;}
		sender.try_send(hand[n as usize].clone()).unwrap();
		return;
	}
}
fn signal_bid(index: u8, bid: u8) {println!("Player {} bid {}", index, bid);}
fn signal_play_card(index: u8, card: Card) {println!("Player {} played {}", index, card);}
fn signal_next_turn(index: u8, bidding: bool) {println!("Player {}'s turn to {}", index, if bidding {"bid"} else {"play a card."});}
fn signal_round_scores(scores: RoundScores) {println!("Scores {:#?}", scores);}
fn signal_game_over() {println!("Game Over");}
fn signal_game_loop_error(error: ServerError) {println!("Server Error: {}", error);}