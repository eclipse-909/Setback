use {
	crate::*,
	futures::{FutureExt, TryFutureExt},
	std::collections::HashMap,
	tokio::sync::{
		oneshot::{Sender, channel},
		mpsc::{UnboundedSender, UnboundedReceiver},
	},
};

#[cfg(target_family = "wasm")]
pub use ws_client::*;

#[cfg(not(target_family = "wasm"))]
pub use ws_server_and_client::*;

	type CorrelationMap = HashMap<u32, Sender<Result<Ssp, NetError>>>;
pub type SspTx = UnboundedSender<(Option<Sender<Result<Ssp, NetError>>>, Ssp)>;
	type SspRx = UnboundedReceiver<(Option<Sender<Result<Ssp, NetError>>>, Ssp)>;
pub type FallbackRx = UnboundedReceiver<Result<Ssp, NetError>>;
	type FallbackTx = UnboundedSender<Result<Ssp, NetError>>;

pub trait ResponseFut = Future<Output=Result<(Payload, u32), NetError>> + 'static;

pub trait WebSocketeer {
	const INIT_CORRELATION_ID: u32;
	
	/// this is how you send all messages
	fn ssp_tx(&self) -> &SspTx;
	/// # Important!
	/// The client must start at 1 and the server must start at 3.
	/// This prevents collisions by making them
	/// multiples of 4 plus 1 and 3 respectively and doesn't wrap to 0.
	fn correlation_id(&mut self) -> &mut u32;

	fn auto_inc_id(&mut self) -> u32 {
		*self.correlation_id() = self.correlation_id().wrapping_add(4);
		*self.correlation_id()
	}

	fn run_rw_handler(
		mut reader: WsReader,
		mut writer: WsWriter,
		mut ssp_rx: SspRx,
		fallback_tx: FallbackTx,
	) -> impl Future<Output=()> {async move {
		// TODO maybe there could be better error handling?
		// Right now it just disconnects the moment something goes wrong.
		let mut correlation_map = CorrelationMap::default();
		loop {
			tokio::select! {
				// reading from ws
				res = ws_next(&mut reader) => {
					let ssp = match res {
						Ok(s) => s,
						Err(e) => {
							_ = fallback_tx.send(Err(e));
							return;
						}
					};
					if ssp.correlation_id.id != 0 && let Some(tx) = correlation_map.remove(&ssp.correlation_id.id) {
						// if it fails to send,
						// it's assumed the receiver was dropped due to the response timing out
						// and it should be treated as such at the call-site
						_ = tx.send(Ok(ssp));
					} else {
						if fallback_tx.send(Ok(ssp)).is_err() {return}
					}
				}
				// writing to ws
				option = ssp_rx.recv() => {
					let Some((tx, ssp)) = option else {return};
					if ssp.correlation_id.id != 0 {
						if ssp.correlation_id.end {
							_ = correlation_map.remove(&ssp.correlation_id.id);
						} else if let Some(tx) = tx {
							_ = correlation_map.insert(ssp.correlation_id.id, tx);
						}
					}
					if let Err(e) = ws_send(&mut writer, &ssp).await {
						if ssp.correlation_id.id != 0 && let Some(tx) = correlation_map.remove(&ssp.correlation_id.id) {
							_ = tx.send(Err(e));
						} else {
							_ = fallback_tx.send(Err(e));
						}
						return;
					}
				}
			}
		}
	}}

	// these functions are not marked async because we don't want to move self into the futures they return

	/// Sends a message and waits for the response.
	/// Returns the payload of the response and a correlation id.
	/// The correlation id is 0 if the other side is not expecting a response.
	/// If the correlation id is not 0, you must send another message.
	fn send_new_request(&mut self, payload: impl Into<Payload>) -> impl ResponseFut {
		self.send_request(payload, 0)
	}

	/// Sends a message and waits for the response.
	/// This is intended to respond to a previous message which is why it takes a correlation id.
	/// To send a new request, use [Self::send_new_request].
	/// Returns the payload of the response and a correlation id.
	/// The correlation id is 0 if the other side is not expecting a response.
	/// If the correlation id is not 0, you must send another message.
	fn send_request(&mut self, payload: impl Into<Payload>, mut correlation_id: u32) -> impl ResponseFut {
		if correlation_id == 0 {
			correlation_id = self.auto_inc_id();
		}
		Self::send_request_raw(self.ssp_tx(), payload, correlation_id)
	}

	/// Correlation id must not be 0!
	fn send_request_raw(ssp_tx: &SspTx, payload: impl Into<Payload>, correlation_id: u32) -> impl ResponseFut {
		let (tx, rx) = channel();
		let send_res = ssp_tx
			.send((Some(tx), Ssp::request(payload, correlation_id)))
			.map_err(|_| NetError::Disconnected);
		rx.map_err(|_| NetError::Disconnected).map(move |result| {
			send_res?;
			let ssp = result.flatten()?;
			debug_assert_eq!(ssp.correlation_id.id, correlation_id);
			Ok((ssp.payload, if ssp.correlation_id.end {0} else {correlation_id}))
		})
	}
	
	/// Sends a message in response to some request from the other side.
	/// This ends a correlation, so if you need to keep sending messages,
	/// use [Self::ws_request].
	fn send_response(&self, payload: impl Into<Payload>, correlation_id: u32) -> Result<(), NetError> {
		debug_assert_ne!(correlation_id, 0);
		Self::send_response_raw(self.ssp_tx(), payload, correlation_id)
	}
	
	fn send_response_raw(ssp_tx: &SspTx, payload: impl Into<Payload>, correlation_id: u32) -> Result<(), NetError> {
		ssp_tx
			.send((None, Ssp::response(payload, correlation_id)))
			.map_err(|_| NetError::Disconnected)
	}
	
	/// A one-off message to send with no expected response.
	fn send_event(&self, payload: impl Into<Payload>) -> Result<(), NetError> {
		Self::send_event_raw(self.ssp_tx(), payload)
	}
	
	fn send_event_raw(ssp_tx: &SspTx, payload: impl Into<Payload>) -> Result<(), NetError> {
		ssp_tx
			.send((None, Ssp::event(payload)))
			.map_err(|_| NetError::Disconnected)
	}
	
	/// Sends an Ack in response if correlation_id is not 0
	fn send_response_ack(&mut self, correlation_id: u32) -> Result<(), NetError> {
		if correlation_id == 0 {return Ok(());}
		self.send_response(Payload::Ack, correlation_id)
	}
}

// The public interface between these two platform-specific
// implementations must be identical on the client side

#[cfg(target_family = "wasm")]
mod ws_client {
	use {
		bytes::BytesMut,
		crate::*,
		futures::{SinkExt, StreamExt, stream::{SplitStream, SplitSink}},
		gloo::net::websocket::{Message, futures::WebSocket},
		std::env,
	};

	pub type WsReader = SplitStream<WebSocket>;
	pub type WsWriter = SplitSink<WebSocket, Message>;

	pub async fn ws_connect() -> Result<(WsReader, WsWriter), NetError> {
		let addr = env::var(ENV_PUBLIC_SERVER_URL).unwrap();
		let (w, r) = WebSocket::open(addr)
			.map_err(|_| NetError::Disconnected)?
			.split();
		Ok((r, w))
	}

	pub async fn ws_send(ws: &mut WsWriter, ssp: &Ssp) -> Result<(), NetError> {
		let bytes = BytesMut::new();
		let bytes = ssp.encode(bytes)?;
		ws.send(Message::Bytes(bytes.to_vec()))
			.await
			.map_err(|_| NetError::Disconnected)
	}

	pub async fn ws_next(ws: &mut WsReader) -> Result<Ssp, NetError> {
		let msg = ws
			.next()
			.await
			.ok_or(NetError::Disconnected)?
			.map_err(|_| NetError::Disconnected)?;
		match msg {
			Message::Bytes(bytes) => Ssp::decode(&bytes),
			_ => Err(NetError::Disconnected),
		}
	}
}

#[cfg(not(target_family = "wasm"))]
mod ws_server_and_client {
	use {
		bytes::BytesMut,
		crate::*,
		std::{env, net::Ipv4Addr},
		sockudo_ws::{SplitReader, SplitWriter, WebSocketStream, Config, handshake,},
		tokio::net::TcpStream,
	};
	#[cfg(not(feature = "server"))]
	use {
		std::sync::Arc,
		tokio_rustls::{rustls, TlsConnector, client::TlsStream},
	};
	
	#[cfg(not(feature = "server"))]
	pub type WsReader = SplitReader<TlsStream<TcpStream>>;
	#[cfg(not(feature = "server"))]
	pub type WsWriter = SplitWriter<TlsStream<TcpStream>>;
	
	#[cfg(feature = "server")]
	pub type WsReader = SplitReader<TcpStream>;
	#[cfg(feature = "server")]
	pub type WsWriter = SplitWriter<TcpStream>;
	
	/// Our payloads should be very small, but we can play around with these values
	/// to try to conserve more memory or give the messages more room if it's too small.
	/// Sizes are in bytes and times are in seconds.
	fn ws_config() -> Config {
		Config {
			max_message_size: 4 * PAGE_SIZE,
			max_frame_size: PAGE_SIZE,
			write_buffer_size: PAGE_SIZE,
			idle_timeout: 4 * NETWORK_TIMEOUT as u32,
			max_backpressure: PAGE_SIZE,
			ping_interval: 2 * NETWORK_TIMEOUT as u32,
			pong_timeout: NETWORK_TIMEOUT as u32,
			close_timeout: NETWORK_TIMEOUT as u32,
			..Default::default()
		}
	}
	
	pub async fn ws_connect() -> Result<(WsReader, WsWriter), NetError> {
		let addr = env::var(ENV_PUBLIC_SERVER_URL).unwrap();
		let stream = TcpStream::connect(addr)
			.await
			.map_err(|_| NetError::Disconnected)?;
		stream.set_nodelay(true)
			.map_err(|_| NetError::Disconnected)?;
		let host = env::var(ENV_SETBACK_HOST).unwrap();
		
		#[cfg(not(feature = "server"))]
		let stream = {
			let mut roots = rustls::RootCertStore::empty();
			roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
			let tls_config = rustls::ClientConfig::builder()
				.with_root_certificates(roots)
				.with_no_client_auth();
			let connector = TlsConnector::from(Arc::new(tls_config));
			let server_name = rustls::pki_types::ServerName::try_from(host.clone())
				.map_err(|_| NetError::Disconnected)?;
			connector.connect(server_name, stream)
				.await
				.map_err(|_| NetError::Disconnected)?
		};
		
		let mut stream = stream;
		handshake::client_handshake(&mut stream, &host, "/", None)
			.await
			.map_err(|_| NetError::Disconnected)?;
		
		Ok(WebSocketStream::client(stream, ws_config()).split())
	}
	
	pub async fn ws_local_connect() -> Result<WebSocketStream<TcpStream>, NetError> {
		let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 42069))
			.await
			.map_err(|_| NetError::Disconnected)?;
		stream.set_nodelay(true)
			.map_err(|_| NetError::Disconnected)?;
		handshake::client_handshake(&mut stream, "localhost", "/", None)
			.await
			.map_err(|_| NetError::Disconnected)?;
		Ok(WebSocketStream::client(stream, ws_config()))
	}
	
	#[cfg(feature = "server")]
	pub async fn ws_accept(mut tcp_stream: TcpStream) -> Result<(WsReader, WsWriter), NetError> {
		tcp_stream.set_nodelay(true)
			.map_err(|_| NetError::Disconnected)?;
		handshake::server_handshake(&mut tcp_stream)
			.await
			.map_err(|_| NetError::Disconnected)?;
		Ok(WebSocketStream::server(tcp_stream, ws_config()).split())
	}
	
	pub async fn ws_send(ws: &mut WsWriter, ssp: &Ssp) -> Result<(), NetError> {
		let bytes = BytesMut::new();
		let bytes = ssp.encode(bytes)?;
		ws.send_binary(bytes.freeze())
			.await
			.map_err(|_| NetError::Disconnected)
		// TODO figure out what happens when ws.send_binary returns Err
	}
	
	pub async fn ws_next(ws: &mut WsReader) -> Result<Ssp, NetError> {
		// TODO figure out what happens when ws.next returns Err or None
		match ws.next()
			.await
			.ok_or(NetError::Disconnected)?
			.map_err(|_| NetError::Disconnected)?
		{
			sockudo_ws::Message::Text(_) => Err(NetError::BadRequest),
			sockudo_ws::Message::Binary(bytes) => Ssp::decode(&bytes),
			sockudo_ws::Message::Ping(_)
			| sockudo_ws::Message::Pong(_)
			=> unreachable!("ping/pong should be handled automatically"),
			sockudo_ws::Message::Close(_) => Err(NetError::Disconnected),
		}
	}
}
