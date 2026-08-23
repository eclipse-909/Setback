#[cfg(target_family = "wasm")]
pub use ws_wasm::*;

#[cfg(not(target_family = "wasm"))]
pub use ws_native::*;

// The public interface between these two platform-specific implementations must be identical

#[cfg(target_family = "wasm")]
mod ws_wasm {
	use {
		crate::{Error, URL_SERVER_WS, ssp::Ssp},
		futures::{SinkExt, StreamExt},
		gloo::net::websocket::{Message, futures::WebSocket as Ws},
	};

	pub type Websocket = Ws;

	pub async fn ws_connect() -> Result<Websocket, Error> {
		Ws::open(URL_SERVER_WS).map_err(|_| Error::NetworkError)
	}

	pub async fn ws_send(ws: &mut Websocket, ssp: &Ssp) -> Result<(), Error> {
		//TODO instead of making (possibly multiple) allocations here
		// we can probably look into using memory pooling
		let mut bytes = Vec::new();
		ssp.encode(&mut bytes)?;
		ws.send(Message::Bytes(bytes))
			.await
			.map_err(|_| Error::NetworkError)
	}

	pub async fn ws_next(ws: &mut Websocket) -> Result<Ssp, Error> {
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
mod ws_native {
	use {
		crate::{Error, URL_SERVER_WS, ssp::Ssp},
		futures::{SinkExt, StreamExt},
		tokio::net::TcpStream,
		tokio_tungstenite::{
			tungstenite::{Bytes, Message},
			*,
		},
	};

	pub type Websocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

	pub async fn ws_connect() -> Result<Websocket, Error> {
		Ok(connect_async(URL_SERVER_WS)
			.await
			.map_err(|_| Error::NetworkError)?
			.0)
	}

	pub async fn ws_send(ws: &mut Websocket, ssp: &Ssp) -> Result<(), Error> {
		//TODO instead of making (possibly multiple) allocations here
		// we can probably look into using memory pooling
		let mut bytes = Vec::new();
		ssp.encode(&mut bytes)?;
		ws.send(Message::Binary(Bytes::from(bytes)))
			.await
			.map_err(|_| Error::NetworkError)
	}

	pub async fn ws_next(ws: &mut Websocket) -> Result<Ssp, Error> {
		let msg = ws
			.next()
			.await
			.ok_or(Error::Disconnected)?
			.map_err(|_| Error::NetworkError)?;
		match msg {
			Message::Binary(bytes) => Ssp::decode(&bytes),
			Message::Close(_) => Err(Error::Disconnected),
			_ => Err(Error::NetworkError),
		}
	}
}
