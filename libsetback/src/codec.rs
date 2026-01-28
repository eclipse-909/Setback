//! The encoder and decoder for sending and receiving [Ssp] over the network.
//! Any changes to this file requires changing at least the minor version of [crate::SETBACK_VERSION] before release.

use {
	crate::{Ssp, Error},
	bytes::{Buf, BufMut, BytesMut},
	tokio_util::codec::{Decoder, Encoder},
};

type Delim = u32;
/// The maximum length an [Ssp] and its [DELIM] should take up in a buffer of bytes.
pub const MAX_LEN: usize = u16::MAX as usize;
const DELIM_SIZE: usize = size_of::<Delim>();
const DELIM: Delim = const {
	let delim: Delim = 0xFEFEFEFE;
	// As long as it isn't UTF-8 it should be good enough.
	// We could make it longer just to be safe.
	match std::str::from_utf8(&delim.to_be_bytes()) {
		Ok(_) => panic!("Delimiter must not be valid UTF-8"),
		_ => {}
	}
	delim
};

// Most of the stuff in [SspDecoder] was stolen from [tokio_util::codec::AnyDelimiterCodec] and modified to fit this program.
pub struct SspDecoder {
	/// Stored index of the next index to examine for the delimiter character.
	/// This is used to optimize searching.
	/// For example, if `decode` was called with `abc` and the delimiter is '{}', it would hold `3`,
	/// because that is the next index to examine.
	/// The next time `decode` is called with `abcde}`, the method will
	/// only look at `de}` before returning.
	next_index: usize,

	/// Are we currently discarding the remainder of a chunk which was over
	/// the length limit?
	is_discarding: bool,
}

impl SspDecoder {
	pub const fn new() -> Self {
		Self {
			next_index: 0,
			is_discarding: false,
		}
	}
}

impl Decoder for SspDecoder {
	type Item = Ssp;
	type Error = crate::Error;

	fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			let read_to = buf.len().min(MAX_LEN + 1);
			let new_chunk_offset = buf[self.next_index..read_to]
				.iter()
				.map_windows(|[a, b, c, d]| [**a, **b, **c, **d])
				.position(|arr| Delim::from_be_bytes(arr) == DELIM);

			match (self.is_discarding, new_chunk_offset) {
				(true, Some(offset)) => {
					// If we found a new chunk, discard up to that offset and
					// then stop discarding. On the next iteration, we'll try
					// to read a chunk normally.
					buf.advance(offset + self.next_index + DELIM_SIZE - 1);
					self.is_discarding = false;
					self.next_index = 0;
				}
				(true, None) => {
					// Otherwise, we didn't find a new chunk, so we'll discard
					// everything we read. On the next iteration, we'll continue
					// discarding up to max_len bytes unless we find a new chunk.
					buf.advance(read_to - DELIM_SIZE + 1);
					self.next_index = 0;
					if buf.is_empty() {
						return Ok(None);
					}
				}
				(false, Some(offset)) => {
					// Found a chunk!
					let new_chunk_index = offset + self.next_index;
					self.next_index = 0;
					let mut chunk = buf.split_to(new_chunk_index + DELIM_SIZE);
					chunk.truncate(chunk.len() - DELIM_SIZE);
					let chunk = postcard::from_bytes(&chunk).map_err(|_| Error::BadEncoding)?;
					return Ok(Some(chunk));
				}
				(false, None) if buf.len() > MAX_LEN => {
					// Reached the maximum length without finding a
					// new chunk, return an error and start discarding on the
					// next call.
					self.is_discarding = true;
					return Err(Error::BadEncoding);
				}
				(false, None) => {
					// We didn't find a chunk or reach the length limit, so the next
					// call will resume searching at the current offset.
					self.next_index = read_to - DELIM_SIZE + 1;
					return Ok(None);
				}
			}
		}
	}
}

pub struct SspEncoder;

impl Encoder<&Ssp> for SspEncoder {
	type Error = Error;

	fn encode(&mut self, chunk: &Ssp, buf: &mut BytesMut) -> Result<(), Self::Error> {
		// The buffers should be initialized with enough capacity which is tested with this assertion.
		// If for whatever reason this isn't true for production,
		// we can just resize the buffer instead of panicking or returning an error.
		debug_assert!(buf.capacity() >= MAX_LEN);
		#[cfg(not(debug_assertions))]
		if Ssp::MAX_LEN > buf.capacity() {
			buf.reserve(Ssp::MAX_LEN - buf.capacity());
		}

		// I assume the only error would be from the buffer being too small
		postcard::to_slice(chunk, buf).map_err(|_| Error::BadEncoding)?;
		buf.put(DELIM.to_be_bytes().as_slice());

		Ok(())
	}
}