use {
	crate::*,
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

	type CorrelationMap = HashMap<u32, Sender<Result<Ssp, Error>>>;
pub type SspTx = UnboundedSender<(Option<Sender<Result<Ssp, Error>>>, Ssp)>;
	type SspRx = UnboundedReceiver<(Option<Sender<Result<Ssp, Error>>>, Ssp)>;
pub type FallbackRx = UnboundedReceiver<Result<Ssp, Error>>;
	type FallbackTx = UnboundedSender<Result<Ssp, Error>>;

pub trait WebSocketeer {
	/// this is how you send all messages
	fn ssp_tx(&self) -> &SspTx;
	/// this is how you read messages that don't (yet) belong to a correlation
	fn fallback_rx(&mut self) -> &mut FallbackRx;
	/// # Important!
	/// The client must start at 1 and the server must start at 3.
	/// This prevents collisions by making them
	/// multiples of 4 plus 1 and 3 respectively and doesn't wrap to 0.
	fn correlation_id(&mut self) -> &mut u32;
	
	fn auto_inc_id(&mut self) -> u32 {
		*self.correlation_id() = self.correlation_id().wrapping_add(4);
		*self.correlation_id()
	}
	
	async fn run_rw_handler(
		mut reader: WsReader,
		mut writer: WsWriter,
		mut ssp_rx: SspRx,
		fallback_tx: FallbackTx,
	) {
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
	}
	
	/// Sends a message and waits for the response.
	/// Returns the payload of the response and a correlation id.
	/// The correlation id is 0 if the other side is not expecting a response.
	/// If the correlation id is not 0, you must send another message.
	async fn send_new_request(&mut self, payload: Payload) -> Result<(Payload, u32), Error> {
		self.send_request(payload, 0).await
	}
	
	/// Sends a message and waits for the response.
	/// This is intended to respond to a previous message which is why it takes a correlation id.
	/// To send a new request, use [Self::send_new_request].
	/// Returns the payload of the response and a correlation id.
	/// The correlation id is 0 if the other side is not expecting a response.
	/// If the correlation id is not 0, you must send another message.
	async fn send_request(&mut self, payload: Payload, mut correlation_id: u32) -> Result<(Payload, u32), Error> {
		if correlation_id == 0 {
			correlation_id = self.auto_inc_id();
		}
		let (tx, rx) = channel();
		self.ssp_tx().send((Some(tx), Ssp::request(payload, correlation_id))).map_err(|_| Error::LogicError)?;
		let ssp = rx.await.map_err(|_| Error::LogicError).flatten()?;
		debug_assert_eq!(ssp.correlation_id.id, correlation_id);
		match ssp.payload {
			Payload::Err(e) => Err(e),
			p => Ok((p, if ssp.correlation_id.end {0} else {correlation_id}))
		}
	}
	
	/// Sends a message in response to some request from the other side.
	/// This ends a correlation, so if you need to keep sending messages,
	/// use [Self::ws_request].
	fn send_response(&mut self, payload: Payload, correlation_id: u32) -> Result<(), Error> {
		debug_assert_ne!(correlation_id, 0);
		self.ssp_tx().send((None, Ssp::response(payload, correlation_id))).map_err(|_| Error::LogicError)
	}
	
	/// A one-off message to send with no expected response.
	fn send_event(&mut self, payload: Payload) -> Result<(), Error> {
		self.ssp_tx().send((None, Ssp::event(payload))).map_err(|_| Error::LogicError)
	}
	
	/// Sends an Ack in response if correlation_id is not 0
	fn send_response_ack(&mut self, correlation_id: u32) -> Result<(), Error> {
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
		crate::{Error, URL_SERVER_WS, ssp::Ssp},
		futures::{SinkExt, StreamExt, stream::{SplitStream, SplitSink}},
		gloo::net::websocket::{Message, futures::WebSocket},
	};

	pub type WsReader = SplitStream<WebSocket>;
	pub type WsWriter = SplitSink<WebSocket, Message>;

	pub async fn ws_connect() -> Result<(WsWriter, WsReader), Error> {
		Ok(WebSocket::open(URL_SERVER_WS).map_err(|_| Error::NetworkError)?.split())
	}

	pub async fn ws_send(ws: &mut WsWriter, ssp: &Ssp) -> Result<(), Error> {
		let bytes = BytesMut::new();
		let bytes = ssp.encode(bytes)?;
		ws.send(Message::Bytes(bytes.to_vec()))
			.await
			.map_err(|_| Error::NetworkError)
	}

	pub async fn ws_next(ws: &mut WsReader) -> Result<Ssp, Error> {
		let msg = ws
			.next()
			.await
			.ok_or(Error::Disconnected)?
			.map_err(|_| Error::NetworkError)?;
		match msg {
			Message::Bytes(bytes) => Ssp::decode(&bytes),
			_ => Err(Error::NetworkError),
		}
	}
}

#[cfg(not(target_family = "wasm"))]
mod ws_server_and_client {
	use {
		bytes::BytesMut,
		crate::*,
		// futures::{SinkExt, StreamExt},
		
		sockudo_ws::{SplitReader, SplitWriter, WebSocketStream, Config},
		tokio::net::TcpStream,
		
	};
	#[cfg(feature = "client")]
	use {
		std::sync::Arc,
		tokio_rustls::{rustls, TlsConnector, client::TlsStream},
		url::Url,
		sockudo_ws::handshake::client_handshake,
	};
	#[cfg(not(feature = "client"))]
	use {
		sockudo_ws::handshake::server_handshake,
	};
	
	#[cfg(feature = "client")]
	pub type WsReader = SplitReader<TlsStream<TcpStream>>;
	#[cfg(feature = "client")]
	pub type WsWriter = SplitWriter<TlsStream<TcpStream>>;
	
	#[cfg(not(feature = "client"))]
	pub type WsReader = SplitReader<TcpStream>;
	#[cfg(not(feature = "client"))]
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
	
	#[cfg(feature = "client")]
	pub async fn ws_connect() -> Result<(WsReader, WsWriter), Error> {
		let url = Url::parse(URL_SERVER_WS).expect("invalid ws url");
		let host = url.host_str().expect("url needs a host").to_string();
		let port = url.port_or_known_default().unwrap_or(443); // 443 for wss
		let path = if url.query().is_some() {
			format!("{}?{}", url.path(), url.query().unwrap())
		} else {
			url.path().to_string()
		};
		
		let tcp = TcpStream::connect((host.as_str(), port)).await.map_err(|_| Error::NetworkError)?;
		tcp.set_nodelay(true).map_err(|_| Error::LogicError)?;
		
		let mut roots = rustls::RootCertStore::empty();
		roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
		let tls_config = rustls::ClientConfig::builder()
			.with_root_certificates(roots)
			.with_no_client_auth();
		let connector = TlsConnector::from(Arc::new(tls_config));
		let server_name = rustls::pki_types::ServerName::try_from(host.clone()).map_err(|_| Error::NetworkError)?;
		let mut tls_stream = connector.connect(server_name, tcp).await.map_err(|_| Error::NetworkError)?;
		
		client_handshake(&mut tls_stream, &host, &path, None).await.map_err(|_| Error::NetworkError)?;
		
		Ok(WebSocketStream::client(tls_stream, ws_config()).split())
	}
	
	#[cfg(not(feature = "client"))]
	pub async fn ws_accept(mut tcp_stream: TcpStream) -> Result<(WsReader, WsWriter), Error> {
		tcp_stream.set_nodelay(true).map_err(|_| Error::ServerError)?;
		server_handshake(&mut tcp_stream).await.map_err(|_| Error::NetworkError)?;
		Ok(WebSocketStream::server(tcp_stream, ws_config()).split())
	}
	
	pub async fn ws_send(ws: &mut WsWriter, ssp: &Ssp) -> Result<(), Error> {
		let bytes = BytesMut::new();
		let bytes = ssp.encode(bytes)?;
		ws.send_binary(bytes.freeze())
			.await
			.map_err(|_| Error::NetworkError)
		// TODO figure out what happens when ws.send_binary returns Err
	}
	
	pub async fn ws_next(ws: &mut WsReader) -> Result<Ssp, Error> {
		// TODO figure out what happens when ws.next returns Err or None
		match ws.next()
			.await
			.ok_or(Error::Disconnected)?
			.map_err(|_| Error::NetworkError)?
		{
			sockudo_ws::Message::Text(_) => Err(Error::BadProtocol),
			sockudo_ws::Message::Binary(bytes) => Ssp::decode(&bytes),
			sockudo_ws::Message::Ping(_)
			| sockudo_ws::Message::Pong(_)
			=> unreachable!("ping/pong should be handled automatically"),
			sockudo_ws::Message::Close(_) => Err(Error::Disconnected),
		}
	}
}
