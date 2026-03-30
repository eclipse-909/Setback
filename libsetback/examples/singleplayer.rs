#[cfg(not(feature = "gpui"))]
use {
	libsetback::{Bid, Card, GameSetup, SmallVec, GameEvent},
	std::io::Write
};

#[cfg(feature = "gpui")]
fn main() {panic!("disable the gpui feature to run this example")}

#[cfg(not(feature = "gpui"))]
#[tokio::main]
async fn main() {
	let log_path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/logs/singleplayer.log"));
	std::fs::create_dir_all(log_path.parent().unwrap()).unwrap();
	let log_file = std::fs::File::create(log_path).unwrap();
	tracing_subscriber::fmt()
		.with_writer(log_file)
		.with_ansi(false)
		.with_level(true)
		.compact()
		.init();
	
	let (cancel_token, mut event_subscriber, prompt_responder) = GameSetup::new_singleplayer(String::from("example"))
		.init()
		.expect("Game cannot start in invalid state")
		.spawn_singleplayer();
	let mut my_hand = SmallVec::new();
	let mut my_idx = u8::MAX;
	let mut input_buf = String::new();
	loop {
		std::io::stdout().flush().unwrap();
		match event_subscriber.recv().await.expect("event subscriber closed unexpectedly") {
			GameEvent::Init {idx} => {
				my_idx = idx;
				println!("Game is starting! You are player {idx}");
			}
			GameEvent::PlayerBid {bid, idx} => {
				if idx == my_idx {
					println!("You bid {}", bid.to_points());
				} else {
					println!("Player {idx} bid {}", bid.to_points());
				}
			}
			GameEvent::PlayerPlayedCard {card, idx} => {
				if idx == my_idx {
					println!("You played the {card:?}");
					my_hand.remove_item(card);
				} else {
					println!("Player {idx} played the {card:?}");
				}
			}
			GameEvent::PromptBid(maybe_err) => {
				if let Some(err) = maybe_err {
					println!("{err} - please try again:");
				} else {
					println!("Here is your current hand: {my_hand:?}\nIt is your turn, please enter your bid:");
				}
				loop {
					readln(&mut input_buf);
					let Some(bid) = Bid::parse_string(&input_buf) else {
						println!("Invalid format - make sure your bid is in one of these formats{}Please try again:", Bid::BID_INPUT_FMT);
						continue;
					};
					prompt_responder.send(GameEvent::PlayerBid { bid, idx: 0 }).expect("prompt response channel closed");
					break;
				}
			}
			GameEvent::PromptCard(maybe_err) => {
				if let Some(err) = maybe_err {
					println!("{err} - please try again:");
				} else {
					println!("Here is your current hand: {my_hand:?}\nIt is your turn, please enter your card:");
				}
				loop {
					readln(&mut input_buf);
					let Some(card_res) = Card::parse_string(&input_buf) else {
						println!("Invalid format - make sure your card is in one of these formats{}Please try again:", Card::CARD_INPUT_FMT);
						continue;
					};
					let card = match card_res {
						Ok(c) => c,
						Err(idx) => {
							if idx >= my_hand.len() {
								println!("Invalid index - index is out of range, please try again:");
								continue;
							}
							my_hand[idx]
						}
					};
					prompt_responder.send(GameEvent::PlayerPlayedCard { card, idx: 0 }).expect("prompt response channel closed");
					break;
				}
			}
			GameEvent::DealCards {hand, idx} => {
				my_hand = hand.clone();
				println!("Player {idx} dealt cards. Your hand: {my_hand:?}");
			}
			GameEvent::RoundScores(round_scores) => println!("{round_scores:?}"),
			GameEvent::GameOver => {
				println!("Game Over! Thanks for playing!");
				return cancel_token.cancel()
			}
		}
	}
}

#[cfg(not(feature = "gpui"))]
fn readln(buf: &mut String) {
	buf.clear();
	_ = std::io::stdin().read_line(buf).expect("Failed to read line from stdin");
	buf.trim().to_string().clone_into(buf);
}