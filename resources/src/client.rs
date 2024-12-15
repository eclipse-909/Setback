use {
	crate::{
		ssp::Ssp,
		util::{ServerError}
	},
	tokio::{
		sync::mpsc::UnboundedSender,
		net::TcpStream,
		io::{AsyncWriteExt, AsyncReadExt}
	},
	bincode::{Decode, Encode, config::standard},
	serde::{Deserialize, Serialize}
};

pub struct Client {
	tcp_stream: TcpStream,
	///transmitter to propagate messages to the main server or bevy_game server in a thread-safe manner
	pub owner_tx: UnboundedSender<Ssp>,
	pub account_id: u32
}

impl Client {
	pub const fn new(tcp_stream: TcpStream, owner_tx: UnboundedSender<Ssp>) -> Self {Self {tcp_stream, owner_tx, account_id: 0}}
	///Sends a Ssp message over TCP to this client on their end device. The first 2 bytes is a u16 representing the length of the rest of the message.
	pub async fn message_client(&mut self, ssp: &Ssp) -> Result<(), ServerError> {
		match bincode::encode_to_vec(ssp, standard()) {
			Ok(mut bytes) => {
				//prefix with 2-byte length
				let mut message: Vec<u8> = (bytes.len() as u16).to_le_bytes().to_vec();
				message.append(&mut bytes);
				if self.tcp_stream.write_all(message.as_slice()).await.is_err() {
					Err(ServerError::TcpSendFailed)
				} else {Ok(())}
			}
			Err(_) => {Err(ServerError::Deserialization)}
		}
	}
	///Assume buffer has a capacity of 1024. May mutate the buffer even if an Error is returned.
	pub async fn read(&mut self, buffer: &mut [u8]) -> Result<u16, std::io::Error> {
		let len: u16 = self.tcp_stream.read_u16_le().await?;
		match self.tcp_stream.read_exact(&mut buffer[..len as usize]).await {
			Ok(num) => {Ok(num as u16)}
			Err(err) => {Err(err)}
		}
	}
	pub async fn shutdown(&mut self) {let _ = self.tcp_stream.shutdown().await;}
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}

#[derive(Serialize, Deserialize)]
pub struct AccountResponse {pub account_id: u32}