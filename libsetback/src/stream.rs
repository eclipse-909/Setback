use {
	crate::{Error, FreeList},
	serde::{Deserialize, Serialize},
	std::{
		ops::{Deref, DerefMut},
		ptr::{copy, copy_nonoverlapping}
	},
	tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split},
	tracing::error,
};

// u16::MAX * 16384 is 1 GiB
pub type MemPool = FreeList<MemBuf, 16384>;

/// This is just a Box to some memory
#[repr(transparent)]
struct MemBuf(Box<[u8; u16::MAX as usize]>);
impl Default for MemBuf {
	fn default() -> Self { Self(unsafe {Box::new_uninit().assume_init()}) }
}
impl Deref for MemBuf {
	type Target = [u8; u16::MAX as usize];
	fn deref(&self) -> &Self::Target { &*self.0 }
}
impl DerefMut for MemBuf {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

/// Creates a buffered+framed reader and writer from an async stream.
///
/// The reader and writer doesn't force one generic type to be serialized,
/// meaning you can send and receive different arbitrary types with the same stream over different send/recv calls.
/// However, this is often not very useful since it's probably best to just use a big enum if you expect different payloads.
/// This design was intentional because constraining generic types to [serde::Deserialize] is lifetime hell,
/// and it's just easier this way.
pub fn new_stream<S: AsyncWriteExt + AsyncReadExt + Unpin>(stream: S) -> (Reader<S>, Writer<S>) {
	let (r, w) = split(stream);
	(
		Reader {
			inner: r,
			buffer: None,
			cursor: 0,
			discarding: false,
		},
		Writer {
			inner: w,
		}
	)
}

/// Calls shutdown when dropped
pub struct Writer<W: AsyncWriteExt + Unpin> {
	inner: WriteHalf<W>,
}

impl<W: AsyncWriteExt + Unpin> Writer<W> {
	/// ## Error
	/// Err may indicate the buffer is too small.
	pub async fn send<T: Serialize>(&mut self, frame: &T, pool: &MemPool) -> Result<(), Error> {
		let mut buffer = pool.take().await;
		let used_buffer = postcard::to_slice_cobs(frame, &mut *buffer)
			.map_err(|_| Error::BadEncoding)?;
		self.inner.write_all(used_buffer).await
			.map_err(|_| Error::NetworkError)?;
		pool.give(buffer).await;
		self.inner.flush().await
			.map_err(|_| Error::NetworkError)
	}
}

impl<W: AsyncWriteExt + Unpin> Drop for Writer<W> {
	fn drop(&mut self) {
		self.inner.shutdown();
	}
}

pub struct Reader<R: AsyncReadExt + Unpin> {
	inner: ReadHalf<R>,
	buffer: Option<MemBuf>,
	/// The index of the first unused byte within self.buffer
	cursor: usize,
	discarding: bool,
}

impl<R: AsyncReadExt + Unpin> Reader<R> {
	/// Returns the next frame in the stream.
	/// This function may take an unlimited amount of time to return, so use timeouts if necessary.
	/// ## Error
	/// If Err is returned, assume the frame has been lost.
	/// ## Cancel Safety
	/// Not cancel safe. If this function is canceled, the buffer may or may not contain corrupt data.
	/// You can use [Self::discard_next] to discard a potentially corrupt frame of data. Doing so means
	/// the frame is lost, but the next recv will be starting over good as new.
	pub async fn recv<T: for<'de> Deserialize<'de>>(&mut self, pool: &MemPool) -> Result<T, Error> {
		// If we want to return the next frame as soon as possible after we get it, we don't want to have to wait
		// for more frames just to fill the buffer. We can inspect the buffer after each call for the delimiter.
		// Then deserialize the frame and return it once we find the delimiter. If there is an error,
		// we want to make sure the next time recv is called, it's not trying to deserialize corrupted data.
		// We may have to discard all the bytes the stream gives us until we hit the delimiter again.

		if self.buffer.is_none() {
			self.cursor = 0;
			self.buffer = Some(pool.take().await);
		}
		let buffer = unsafe {self.buffer.as_mut().unwrap_unchecked()};
		if self.discarding {
			loop {
				let num_bytes_read = self.inner.read(&mut **buffer).await
					.map_err(|_| Error::NetworkError)?;
				if num_bytes_read == 0 { return Err(Error::Disconnected); }
				//TODO: check to see if iter().position() is vectorized
				// It might not be because num_bytes_read is a runtime value.
				// This should be optimized to use simd if possible.
				let Some(delim_idx) = buffer[..num_bytes_read].iter().rev().position(|b| *b == 0) else { continue; };
				if delim_idx == buffer.len() {
					// If the last byte in the buffer happens to be the delimiter,
					// we don't need to memcpy
					std::hint::cold_path();
					self.discarding = false;
					self.cursor = 0;
					break;
				}

				// Shift to-be-used bytes to the beginning of the buffer
				// Example:
				// [3, 5, 7, 1, 0, 4, 2, 6]
				// num_bytes_read = 8
				// delim_idx = 4
				// new_cursor = 3
				// num_discarding = 5
				// num_saving = new_cursor = 3
				// [4, 2, 6, 1, 0, 4, 2, 6]

				let num_discarding = delim_idx + 1;
				let new_cursor = num_bytes_read - num_discarding;
				let buf_ptr = buffer.as_mut_ptr();
				unsafe {
					let buf_ptr_first_used = buf_ptr.add(num_discarding);
					if new_cursor <= num_discarding {
						copy_nonoverlapping(buf_ptr_first_used, buf_ptr, new_cursor);
					} else {
						copy(buf_ptr_first_used, buf_ptr, new_cursor);
					}
				}
				self.discarding = false;
				self.cursor = new_cursor;
				break;
			}
		}
		let mut give_buf = false;
		loop {
			let num_bytes_read = self.inner.read(&mut buffer[self.cursor..]).await
				.map_err(|_| {
					self.discarding = true;
					Error::NetworkError
				})?;
			if num_bytes_read == 0 { return Err(Error::Disconnected); }
			let new_cursor = self.cursor + num_bytes_read;
			if buffer[new_cursor - 1] == 0 {
				give_buf = true;
				self.cursor = new_cursor;
				break;
			}
			let delim_found = buffer[self.cursor..new_cursor - 1].contains(&0);
			self.cursor = new_cursor;
			if delim_found { break; }
			if self.cursor == buffer.len() {
				error!("The frame is too large or the buffer is too small. Buffer length: {}", u16::MAX);
				self.discarding = true;
				return Err(Error::IndexOutOfBounds);
			}
		}
		let buf_ptr = buffer.as_mut_ptr();
		let (frame, unused_buffer) = postcard::take_from_bytes_cobs(&mut buffer[..self.cursor])
			.map_err(|_| {
				self.discarding = true;
				Error::BadEncoding
			})?;
		// Shift to-be-used bytes to the beginning of the buffer
		unsafe {
			if self.cursor >= unused_buffer.len() {
				copy_nonoverlapping(unused_buffer.as_ptr(), buf_ptr, unused_buffer.len());
			} else {
				copy(unused_buffer.as_ptr(), buf_ptr, unused_buffer.len());
			}
		}
		self.cursor = unused_buffer.len();
		if give_buf {
			// If the last byte in the buffer happens to be the delimiter,
			// we can release the buffer back to the pool
			pool.give(unsafe {self.buffer.take().unwrap_unchecked()});
		}
		Ok(frame)
		// omg this function got so big and complicated so fast
	}

	/// The next time recv is called, the next frame (which may or may not be corrupted data) will be lost,
	/// and it will try to return the next frame after that.
	pub fn discard_next(&mut self) {
		self.discarding = true;
	}
}