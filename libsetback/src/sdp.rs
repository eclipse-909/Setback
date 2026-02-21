use {
	crate::{Error, SDP_VERSION, Version},
	serde::{Deserialize, Serialize},
};

///Setback Daemon Protocol
#[derive(Serialize, Deserialize)]
pub struct Sdp {
	/// The version of the sender
	pub version: Version,
	pub payload: Payload,
}

#[derive(Serialize, Deserialize)]
pub enum Payload {
	Err(Error),
	NotifyStart(StartInfo),
	PsReq,
	PsRes(PsInfo),
}

impl Sdp {
	pub const fn new(payload: Payload) -> Self {
		Self {
			version: SDP_VERSION,
			payload,
		}
	}

	pub fn serialize<'b>(&self, buf: &'b mut [u8]) -> Result<&'b mut [u8], Error> {
		postcard::to_slice_cobs(self, buf)
			.map_err(|_| Error::BadEncoding)
	}

	pub fn deserialize(buf: &mut [u8]) -> Result<Self, Error> {
		postcard::from_bytes_cobs(buf)
			.map_err(|_| Error::BadEncoding)
	}
}

#[derive(Serialize, Deserialize)]
pub struct StartInfo {
	pub port: u16,
	pub version: Version,
	pub addr: String,
}

#[derive(Serialize, Deserialize)]
pub struct PsInfo {}