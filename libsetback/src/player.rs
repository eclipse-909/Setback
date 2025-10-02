use {
	crate::{
		constants::*,
		items::*,
		connection::*
	},
	bincode::{
		BorrowDecode,
		Decode,
		Encode,
		de::{BorrowDecoder, Decoder},
		enc::Encoder,
		error::{DecodeError, EncodeError}
	}
};

#[derive(Encode, Decode, Debug, Clone)]
pub struct Player {
	pub player_type: PlayerType,
	pub username: String,
	pub hand: Vec<Card>,
	pub team_id: u8
}

impl Player {
	pub fn new_singleplayer_self(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::SingleplayerSelf,
			//TODO load username from disk
			username: String::from("Me"),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}

	///Empty slot for a human player in multiplayer. This function should only be called by the server.
	pub fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanEmpty,
			username: String::from("Waiting for player to connect..."),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}

	pub fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::AI,
			username: String::from("Bot"),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}

	pub fn can_follow(&self, lead: Suit) -> bool { self.hand.iter().any(|card| {card.suit == lead}) }
}

#[derive(Debug)]
pub enum PlayerType {
	///Yourself in singleplayer.
	SingleplayerSelf,
	///A slot for a human player in multiplayer that doesn't have a connection yet. This is only used by the server.
	HumanEmpty,
	///A connected player in multiplayer. This is only used by the server.
	HumanConnection(ClientConnection),
	///The account id and ready status of a connected player in multiplayer.
	///The server will convert a HumanConnection to a MultiplayerHuman when serializing and sending it to clients.
	///The client doesn't store this, they just take the account id and ready status from the HumanConnection and update the UI accordingly.
	MultiplayerHuman{account_id: u32, ready: bool},
	AI
}

impl PlayerType {
	pub fn set(&mut self, other: Self) {
		if let Self::HumanConnection(connection) = self && let Self::MultiplayerHuman{account_id, ready} = other {
			connection.account_id = account_id;
			connection.ready = ready;
			return;
		}
		*self = other;
	}
}

impl Clone for PlayerType {
	fn clone(&self) -> Self {
	    match self {
			Self::SingleplayerSelf => Self::SingleplayerSelf,
			Self::HumanEmpty => Self::HumanEmpty,
			//HumanConnection is cloned as MultiplayerHuman
			Self::HumanConnection(connection) => Self::MultiplayerHuman {
				account_id: connection.account_id,
				ready: connection.ready
			},
			Self::MultiplayerHuman{account_id, ready} => Self::MultiplayerHuman {
				account_id: *account_id,
				ready: *ready
			},
			Self::AI => Self::AI
		}
	}
}

impl Encode for PlayerType {
	fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
		match self {
			Self::SingleplayerSelf => {Encode::encode(&0, encoder)}
			Self::HumanEmpty => {Encode::encode(&1, encoder)}
			Self::HumanConnection(connection) => {
				//HumanConnection is encoded as MultiplayerHuman
				Encode::encode(&3, encoder)?;
				Encode::encode(&connection.account_id, encoder)?;
				Encode::encode(&connection.ready, encoder)
			}
			Self::MultiplayerHuman{account_id, ready} => {
				Encode::encode(&3, encoder)?;
				Encode::encode(&account_id, encoder)?;
				Encode::encode(&ready, encoder)
			}
			Self::AI => {Encode::encode(&4, encoder)}
		}
	}
}

impl<Context> Decode<Context> for PlayerType {
	fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
		let player_type: u8 = Decode::decode(decoder)?;
		match player_type {
			0 => Ok(Self::SingleplayerSelf),
			1 => unreachable!("HumanConnection is encoded as MultiplayerHuman"),
			2 => Ok(Self::HumanEmpty),
			3 => Ok(Self::MultiplayerHuman{account_id: Decode::decode(decoder)?, ready: Decode::decode(decoder)?}),
			4 => Ok(Self::AI),
			_ => unreachable!("Invalid player type: {}", player_type)
		}
	}
}

impl<'de, Context> BorrowDecode<'de, Context> for PlayerType {
	fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
		let player_type: u8 = Decode::decode(decoder)?;
		match player_type {
			0 => Ok(Self::SingleplayerSelf),
			1 => unreachable!("HumanConnection is encoded as MultiplayerHuman"),
			2 => Ok(Self::HumanEmpty),
			3 => Ok(Self::MultiplayerHuman{account_id: BorrowDecode::borrow_decode(decoder)?, ready: BorrowDecode::borrow_decode(decoder)?}),
			4 => Ok(Self::AI),
			_ => unreachable!("Invalid player type: {}", player_type)
		}
	}
}