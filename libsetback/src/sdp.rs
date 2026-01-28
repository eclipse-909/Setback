use {
	crate::Version,
	serde::{Serialize, Deserialize}
};

///Setback Daemon Protocol
#[derive(Serialize, Deserialize)]
pub struct Sdp {
	/// The version of the sender
	version: Version,
	payload: Payload,
}

#[derive(Serialize, Deserialize)]
pub enum Payload {
	TcpPort(u16),
}