use {
	// shared::{Card, GameEvent, SetupData, GameRuntime},
	libsetback::{Card, GameEvent, SetupData, GameRuntime},
	std::io::{stdin, stdout, Write},
	tokio::sync::mpsc::Sender
};

#[tokio::main]
async fn main() {
	let setup: SetupData = SetupData::new_singleplayer();
	let mut game_runtime: GameRuntime = setup.start_game();
	tokio::select! {
		_ = async {
			loop {
				match game_runtime.event_emitter.recv().await {
					None => {
						game_runtime.cancel_token.cancel();
						return;
					}
					Some(GameEvent::PlayerBid{index, amount}) => {
						println!("Player {} bid {}", index, amount);
					}
					Some(GameEvent::PlayerPlayedCard{index, card}) => {
						println!("Player {} played {}", index, card);
					}
					Some(GameEvent::PlayerTurn{index}) => {
						println!("Player {}'s turn", index);
					}
					Some(GameEvent::PromptBid{index, hand}) => {
						let sender: &Sender<u8> = &game_runtime.prompt_senders[index as usize].0;
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
					Some(GameEvent::PromptCard{index, hand}) => {
						let sender: &Sender<Card> = &game_runtime.prompt_senders[index as usize].1;
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
							sender.try_send(hand[n as usize]).unwrap();
							return;
						}
					}
					Some(GameEvent::DealCards) => {
						println!("Cards are being dealt.");
					}
					Some(GameEvent::RoundScores(scores)) => {
						println!("Round scores: {:#?}", scores);
					}
					Some(GameEvent::GameOver(scores)) => {
						println!("Game over: {:#?}", scores);
						game_runtime.cancel_token.cancel();
						return;
					}
					Some(GameEvent::GameError(error)) => {
						println!("Game error: {}", error);
					}
				}
			}
		} => {}
		_ = game_runtime.cancel_token.cancelled() => {}
	}
}