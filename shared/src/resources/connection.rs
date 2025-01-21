use {
	super::{
		ssp::Ssp,
		util::ClientError
	},
	std::{
		net::{Ipv4Addr, SocketAddrV4},
		future::Future
	},
	tokio::{
		net::{
			TcpStream,
			tcp::{OwnedWriteHalf, OwnedReadHalf}
		},
		io::{AsyncWriteExt, AsyncReadExt},
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken,
	bincode::{
		Decode, Encode, config::standard
	},
	serde::{Deserialize, Serialize}
};

pub const SERVER_SOCKET: SocketAddrV4 = if cfg!(debug_assertions) {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 42069)
} else {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 42069)//TODO change this to actual socket once server is deployed
};

///This standardizes the method of communication between the server and clients by putting the logic in one place.
pub trait SetbackConnection: Send {
	const BUF_LEN: usize = size_of::<Ssp>();//TODO this might not be enough for protocols involving Vec<T>
	fn write_half(&mut self) -> &mut OwnedWriteHalf;
	fn send(&mut self, ssp: &Ssp) -> impl Future<Output = Result<(), std::io::Error>> + Send {async move {
		let mut bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).map_err(|err|std::io::Error::other(err))?;
		//prefix with 2-byte length
		let mut message: Vec<u8> = (bytes.len() as u16).to_le_bytes().to_vec();
		message.append(&mut bytes);
		self.write_half().write_all(message.as_slice()).await
	}}
	fn read(read_half: &mut OwnedReadHalf, buffer: &mut [u8]) -> impl Future<Output = Result<Ssp, std::io::Error>> + Send {async move {
		let len: u16 = read_half.read_u16_le().await?;
		debug_assert!(len <= Self::BUF_LEN as u16, "Buffer not big enough. Length of this message: {} bytes.", len);
		read_half.read_exact(&mut buffer[..len as usize]).await?;
		Ssp::decode(&mut buffer[..len as usize]).map_err(|err|std::io::Error::other(err))
	}}
}

///Connection to the server, owned by the client.
pub struct ServerConnection {
	write_half: OwnedWriteHalf,
	read_handle: JoinHandle<()>,
	cancellation_token: CancellationToken
}

impl ServerConnection {
	///None means it couldn't connect to the server
	async fn connect(ssp_handler: fn(msg: Result<Ssp, ClientError>)) -> Option<Self> {
		let (mut read_half, write_half) = TcpStream::connect(SERVER_SOCKET).await.ok()?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let cloned_token: CancellationToken = cancellation_token.clone();
		let read_handle: JoinHandle<()> = tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: [u8; <ServerConnection as SetbackConnection>::BUF_LEN] = [0; <ServerConnection as SetbackConnection>::BUF_LEN];
				loop {
					let res: Result<Ssp, ClientError> = Self::read(&mut read_half, &mut buffer).await.map_err(|_|ClientError::IO);
					let is_err: bool = res.is_err();
					ssp_handler(res);
					if is_err {break;}
				}
				//TODO do I need to handle the errors here? If so, how?
			} => {}
			_ = cloned_token.cancelled() => {}
		}});
		Some(Self {write_half, read_handle, cancellation_token})
	}
}

impl SetbackConnection for ServerConnection {
	#[inline(always)]
	fn write_half(&mut self) -> &mut OwnedWriteHalf {&mut self.write_half}
}

impl Drop for ServerConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
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