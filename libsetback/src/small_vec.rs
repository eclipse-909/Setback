use {
	std::{
		marker::PhantomData,
		mem::{MaybeUninit, ManuallyDrop},
		ops::{Index, IndexMut},
		fmt
	},
	serde::{
		Deserialize, Deserializer, Serialize, Serializer,
		ser::SerializeSeq,
		de::{SeqAccess, Visitor}
	}
};

/// SmallVec is an array that lives on the stack. It uses familiar vector operations for changing the length at runtime.
/// The difference being the capacity must be known at compile time and cannot change, and you must not add items past the capacity.
/// ## Safety
/// In Debug mode, functions will panic at runtime if you fuck shit up.
/// In Release mode, functions have undefined behavior and aren't marked unsafe because that's dumb and verbose.
pub struct SmallVec<T, const CAPACITY: usize> {
	arr: [MaybeUninit<T>; CAPACITY],
	/// Represents the number of consecutive elements in arr that exist starting from index 0.
	len: u8,
}

impl <T, const CAPACITY: usize> SmallVec<T, CAPACITY> {
	pub const fn new() -> Self {
		const {assert!(CAPACITY <= u8::MAX as usize, "CAPACITY must be less than 256");}
		Self {
			arr: MaybeUninit::uninit().transpose(),
			len: 0
		}
	}

	#[inline(always)]
	pub const fn len(&self) -> u8 { self.len }

	#[inline(always)]
	pub const fn empty(&self) -> bool { self.len == 0 }

	pub const fn push(&mut self, val: T) {
		let len = self.len as usize;
		debug_assert!(len < CAPACITY, "Index out of bounds");
		// SAFETY
		// array bounds check is skipped in release mode
		unsafe {self.arr.as_mut_ptr().add(len).write(MaybeUninit::new(val))};
		self.len += 1;
	}

	/// ## SAFETY
	/// It is undefined behavior if a == b in Release mode. It panics in Debug mode.
	pub const fn swap(&mut self, a: u8, b: u8) {
		debug_assert!(a != b, "Indices should not be the same.");
		debug_assert!(a < self.len && b < self.len, "Index out of bounds.");
		unsafe {std::ptr::swap(self.arr.as_mut_ptr().add(a as usize), self.arr.as_mut_ptr().add(b as usize))};
	}

	/// Takes an element out from index and reinserts it at new_index, shifting elements in between if necessary.
	/// ## SAFETY
	/// It is undefined behavior if index == new_index in Release mode. It panics in Debug mode.
	pub const fn reinsert(&mut self, index: u8, new_index: u8) {
		let len = self.len as usize;
		let index = index as usize;
		let new_index = new_index as usize;
		debug_assert!(len <= CAPACITY && index <= len && new_index <= len, "Index out of bounds");
		let ptr = self.arr.as_mut_ptr();
		unsafe {
			let ptr_index = ptr.add(index);
			let ptr_new_index = ptr.add(new_index);
			let item = ptr_index.read();
			if index < new_index {
				// shift left
				std::ptr::copy(
					ptr.add(index + 1),
					ptr_index,
					new_index - index,
				);
			} else if index > new_index {
				// shift right
				std::ptr::copy(
					ptr_new_index,
					ptr.add(new_index + 1),
					index - new_index,
				);
			} else {
				debug_assert!(index != new_index, "index and new_index should not be the same.");
				#[cfg(not(debug_assertions))]
				std::hint::unreachable_unchecked();
			}
			ptr_new_index.write(item);
		}
	}

	/// Moves all elements from other into self.
	/// ## SAFETY
	/// - self and other must not overlap
	/// - self.len + other.len must be <= CAPACITY
	pub const fn append<const N: usize>(&mut self, other: &mut SmallVec<T, N>) {
		debug_assert!(self.len + other.len <= CAPACITY as u8, "Index out of bounds");
		unsafe {std::ptr::copy_nonoverlapping(
			other.arr.as_ptr(),
			self.arr.as_mut_ptr().add(self.len as usize),
			other.len as usize
		)}
		self.len += other.len;
		other.len = 0;
	}

	pub const fn remove(&mut self, index: u8) -> T {
		let len = self.len as usize;
		let index = index as usize;
		unsafe {
			let ptr_index = self.arr.as_mut_ptr().add(index);
			let rm = std::ptr::read(ptr_index).assume_init();
			if index < len - 1 {
				std::ptr::copy(
					ptr_index.add(1),
					ptr_index,
					len - index - 1,
				)
			}
			self.len -= 1;
			rm
		}
	}

	/// Removes and returns the last N elements from self.
	/// ## Safety
	/// It is undefined behavior if N > self.len() in release mode
	pub const fn split<const N: usize>(&mut self) -> SmallVec<T, N> {
		debug_assert!(self.len >= N as u8, "Index out of bounds");
		let mut vec = SmallVec::new();
		unsafe {
			std::ptr::copy_nonoverlapping(
				self.arr.as_ptr().add(self.len as usize - N),
				vec.arr.as_mut_ptr(),
				N,
			);
			self.len -= N as u8;
			vec.len += N as u8;
			vec
		}
	}

	pub const fn iter(&self) -> core::slice::Iter<T> {
		self.as_slice().iter()
	}

	pub const fn iter_mut(&mut self) -> core::slice::IterMut<T> {
		self.as_slice_mut().iter_mut()
	}

	pub const fn as_slice(&self) -> &[T] {
		unsafe {std::slice::from_raw_parts(self.arr.as_ptr().cast(), self.len as usize)}
	}

	pub const fn as_slice_mut(&mut self) -> &mut [T] {
		unsafe {std::slice::from_raw_parts_mut(self.arr.as_mut_ptr().cast(), self.len as usize)}
	}

	/// Unstable sort
	pub fn sort(&mut self) where T: Ord {
		self.as_slice_mut().sort_unstable();
	}

	pub fn dedup(&mut self) where T: PartialEq {
		if self.len <= 1 {return;}
		let mut write = 0;
		for read in 1..self.len {
			if self[read] != self[write] {
				write += 1;
				if read == write {continue;}
				unsafe {
					let ptr: *mut T = self.arr.as_mut_ptr().add(write as usize).cast();
					ptr.drop_in_place();
					std::ptr::copy_nonoverlapping(self.arr.as_ptr().add(read as usize).cast(), ptr, 1);
				}
			}
		}
		self.len = write + 1;
	}
}

impl<T, const CAPACITY: usize> IntoIterator for SmallVec<T, CAPACITY> {
	type Item = T;
	type IntoIter = IntoIter<T, CAPACITY>;
	fn into_iter(self) -> Self::IntoIter {
		IntoIter {
			index: 0,
			small_vec: ManuallyDrop::new(self)
		}
	}
}

impl<T: Clone, const CAPACITY: usize> Clone for SmallVec<T, CAPACITY> {
	fn clone(&self) -> Self {
		let mut new_vec = Self::new();
		for item in self.iter() {
			new_vec.push(item.clone());
		}
		new_vec
	}
}

impl<T: fmt::Debug, const CAPACITY: usize> fmt::Debug for SmallVec<T, CAPACITY> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list().entries(self.iter()).finish()
	}
}

impl<T, const CAPACITY: usize> Index<u8> for SmallVec<T, CAPACITY> {
	type Output = T;

	fn index(&self, index: u8) -> &Self::Output {
		debug_assert!(index < self.len, "Index out of bounds");
		unsafe {self.arr.get_unchecked(index as usize).assume_init_ref()}
	}
}

impl<T, const CAPACITY: usize> IndexMut<u8> for SmallVec<T, CAPACITY> {
	fn index_mut(&mut self, index: u8) -> &mut Self::Output {
		debug_assert!(index < self.len, "Index out of bounds");
		unsafe {self.arr.get_unchecked_mut(index as usize).assume_init_mut()}
	}
}

impl<T, const CAPACITY: usize> FromIterator<T> for SmallVec<T, CAPACITY> {
	fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
		let mut vec = Self::new();
		for item in iter {
			vec.push(item);
		}
		vec
	}
}

impl<T, const CAPACITY: usize> Drop for SmallVec<T, CAPACITY> {
	fn drop(&mut self) {
		for i in 0..self.len {
			unsafe {(*self.arr.as_mut_ptr().add(i as usize)).assume_init_drop()};
		}
	}
}

impl<T: Serialize, const CAPACITY: usize> Serialize for SmallVec<T, CAPACITY> {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_seq(Some(self.len as usize))?;
		for item in self.iter() {
			state.serialize_element(item)?;
		}
		state.end()
	}
}

impl<'de, T: Deserialize<'de>, const CAPACITY: usize> Deserialize<'de> for SmallVec<T, CAPACITY> {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		deserializer.deserialize_seq(SmallVecVisitor { _p: PhantomData })
	}
}

struct SmallVecVisitor<T, const CAPACITY: usize> { _p: PhantomData<[T; CAPACITY]> }

impl<'de, T: Deserialize<'de>, const CAPACITY: usize> Visitor<'de> for SmallVecVisitor<T, CAPACITY> {
	type Value = SmallVec<T, CAPACITY>;

	fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("a sequence")
	}

	fn visit_seq<B: SeqAccess<'de>>(self, mut seq: B) -> Result<Self::Value, B::Error> {
		let Some(size) = seq.size_hint() else {
			return Err(serde::de::Error::custom("SmallVec sequence missing size hint"));
		};
		if size > CAPACITY {
			return Err(serde::de::Error::invalid_length(size, &const_format_args!("{CAPACITY}").as_str().unwrap_or_else(|| unreachable!())));
		}
		let mut values = SmallVec::new();
		while let Some(value) = seq.next_element()? {
			values.push(value);
		}
		Ok(values)
	}
}

/// An iterator over a [SmallVec].
/// ## Safety
/// If the program panics before the iterator is fully consumed in Release mode,
/// the remaining items will not have drop called on them.
pub struct IntoIter<T, const CAPACITY: usize> {
	//TODO: use a pointer to the next item instead of index to avoid pointer arithmetic every time
	index: u8,
	small_vec: ManuallyDrop<SmallVec<T, CAPACITY>>,
}

impl<T, const CAPACITY: usize> Iterator for IntoIter<T, CAPACITY> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		if self.index == self.small_vec.len {return None;}
		let item = unsafe {(*self.small_vec.arr.as_ptr().add(self.index as usize)).assume_init_read()};
		self.index += 1;
		Some(item)
	}
}

#[cfg(debug_assertions)]
impl<T, const CAPACITY: usize> Drop for IntoIter<T, CAPACITY> {
	/// The only reason this is necessary is if the program panics before the iterator is fully consumed.
	/// It is only implemented in Debug mode just so we can debug if there are any problems.
	/// We don't need to waste time in Release mode dropping things if the program has already crashed.
	fn drop(&mut self) {
		for i in self.index..self.small_vec.len {
			unsafe {(*self.small_vec.arr.as_mut_ptr().add(i as usize)).assume_init_drop()};
		}
	}
}

#[cfg(test)]
#[test]
fn test_malicious_small_vec_serde() {
	type TestVec = SmallVec<u8, 1>;
	let mut vec = TestVec::new();

	// len is private so this should never happen.
	// Messing with len is undefined behavior.
	// Lets see if we can break serialization to mimic malicious input.
	vec.len = 1;

	let mut buffer = [0; size_of::<TestVec>()];
	let bytes = postcard::to_slice(&vec, &mut buffer).unwrap();

	let res: TestVec = postcard::from_bytes(bytes).unwrap();
	assert_eq!(res.len, 0);
}