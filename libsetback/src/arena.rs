use {
	crate::PAGE_SIZE,
	std::{
		alloc::{AllocError, Allocator, Layout},
		cell::{Cell, UnsafeCell},
		marker::PhantomData,
		mem::MaybeUninit,
		ops::Deref,
		ptr::NonNull,
	},
	tracing::debug,
};

/// Arena allocator which grows dynamically as you allocate more to it.
/// All memory is freed when the Arena is dropped or cleared, and drop is called on each item allocated.
pub struct Arena<'arena> {
	allocator: ArenaAlloc<'arena>,
}

impl<'arena> Arena<'arena> {
	/// Creates an empty [Arena].
	pub const fn new() -> Self {
		Self {
			allocator: ArenaAlloc {
				_phantom: PhantomData,
				segments: UnsafeCell::new(Vec::new()),
				offset: Cell::new(0),
				cap: 0,
			}
		}
	}

	/// Creates an empty [Arena] with an initial capacity of at least the given amount.
	pub fn new_with_capacity(bytes: usize) -> Self {
		Self {
			allocator: ArenaAlloc {
				_phantom: PhantomData,
				segments: UnsafeCell::new([new_mem_chunk(bytes)].into()),
				offset: Cell::new(0),
				cap: bytes,
			}
		}
	}

	/// Allocates at least enough space for object and moves object into that space.
	/// Returns a reference to object whose lifetime is the same as the arena.
	/// # Panic
	/// Panics if the underlying heap allocation fails (if your system runs out of memory or something similar).
	pub fn alloc<T>(&self, object: T) -> &'arena T {
		// SAFETY:
		// memory is initialized with object
		unsafe {
			let p = self.allocator.allocate(Layout::new::<T>()).unwrap().cast();
			p.write(object);
			p.as_ref()
		}
	}
	
	/// Allocates at least enough space for object and moves object into that space.
	/// Returns a reference to object whose lifetime is the same as the arena.
	/// # Panic
	/// Panics if the underlying heap allocation fails (if your system runs out of memory or something similar).
	#[allow(clippy::mut_from_ref)]
	pub fn alloc_mut<T>(&self, object: T) -> &'arena mut T {
		// SAFETY:
		// memory is initialized with object
		unsafe {
			let mut p = self.allocator.allocate(Layout::new::<T>()).unwrap().cast();
			p.write(object);
			p.as_mut()
		}
	}
	
	/// Allocates at least bytes.
	/// Returns a reference to uninitialized memory whose lifetime is the same as the arena.
	/// # Panic
	/// Panics if the underlying heap allocation fails (if your system runs out of memory or something similar).
	/// # Safety
	/// You must initialize the memory before reading it
	pub fn alloc_uninit<T>(&self, bytes: usize) -> NonNull<T> {
		self.allocator
			.allocate(Layout::from_size_align(bytes, 1).unwrap())
			.unwrap()
			.cast()
	}
	
	/// Allocates at least enough space for object and moves object into that space.
	/// Returns a Box to object whose lifetime is the same as the arena.
	pub fn boxed<T>(&'arena self, object: T) -> Box<T, &'arena ArenaAlloc<'arena>> {
		Box::new_in(object, self)
	}
	
	/// Returns an empty Vec whose lifetime is the same as the arena that contains its elements.
	pub fn vec<T>(&'arena self) -> Vec<T, &'arena ArenaAlloc<'arena>> {
		Vec::new_in(self)
	}
	
	/// Returns an empty Vec with initial capacity whose lifetime is the same as the arena that contains its elements.
	/// Capacity is the number of elements, not the number of bytes.
	pub fn vec_with_capacity<T>(&'arena self, capacity: usize) -> Vec<T, &'arena ArenaAlloc<'arena>> {
		Vec::with_capacity_in(capacity, self)
	}
	
	/// Copies the string slice into the arena and returns the arena-allocated string slice.
	pub fn string(&'arena self, str: &str) -> &'arena str {
		// SAFETY:
		// p is initialized via copy_nonoverlapping.
		// str and p do not overlap because str already existed and p was just allocated.
		// from_raw_parts should hopefully work.
		// str is already utf-8 valid therefore p is.
		unsafe {
			let len = str.len();
			let p = self.alloc_uninit(len).as_ptr();
			std::ptr::copy_nonoverlapping(str.as_ptr(), p, len);
			let s = std::slice::from_raw_parts(p, len);
			str::from_utf8_unchecked(s)
		}
	}
	
	/// Invalidates all reference to this arena and de-allocates most memory.
	/// The last segment in the arena is reused to start the arena.
	pub fn clear(&mut self) {
		if let Some(tail) = self.allocator.segments.get_mut().pop() {
			*self = Self {
				allocator: ArenaAlloc {
					_phantom: PhantomData,
					segments: UnsafeCell::new([tail].into()),
					offset: Cell::new(0),
					cap: 0,
				}
			}
		}
	}
}

impl<'arena> Deref for Arena<'arena> {
	type Target = ArenaAlloc<'arena>;
	
	fn deref(&self) -> &Self::Target {
		&self.allocator
	}
}

impl<'arena, T> AsRef<T> for Arena<'arena>
where
	T: ?Sized,
	<Self as Deref>::Target: AsRef<T>,
{
	fn as_ref(&self) -> &T {
		self.deref().as_ref()
	}
}

/// Fat pointer to the heap-allocated memory.
type ArenaSegment = Box<[MaybeUninit<u8>]>;

fn new_mem_chunk(cap: usize) -> ArenaSegment {
	Box::new_uninit_slice((cap / PAGE_SIZE + 1) * PAGE_SIZE)
}

/// The allocator used for [Arena]. This struct is wrapped by [Arena] to hide implementation details and unnecessary functions.
pub struct ArenaAlloc<'arena> {
	/// Make sure this is the first field so the lifetime ends before the memory is deallocated when the arena is dropped.
	_phantom: PhantomData<&'arena mut ()>,
	// Contains all the memory allocated by the arena in a linked list of memory chunks.
	segments: UnsafeCell<Vec<ArenaSegment>>,
	/// Index within the tail segment to the first unused byte. Offset = 0 is guaranteed to be aligned in memory.
	offset: Cell<usize>,
	/// Total capacity of the arena
	cap: usize,
}

// SAFETY:
// dw
unsafe impl<'arena> Allocator for ArenaAlloc<'arena> {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		// Get the layout alignment to insert padding before the actual location of the object
		let size = layout.size();
		let align = layout.align();
		let offset = self.offset.get();
		let remainder = offset % align;
		if remainder != 0 {
			self.offset.set(offset + (align - remainder));
		}

		// SAFETY:
		// We aren't accessing self.segments anywhere else.
		// We aren't mutating it outside the &mut it gives us.
		let segments = unsafe {self.segments.as_mut_unchecked()};
		
		// If the space needed for this allocation is greater than the amount of memory we have,
		let new_seg_size = if let Some(tail) = segments.last() && self.offset.get() + size > tail.len() {
			// create a new segment with the size of the entire arena
			let new_seg_size = size + self.cap;
			debug!("Increasing arena size to {}", new_seg_size + self.cap);
			self.offset.set(0);
			new_seg_size
		} else {
			// create the first segment in the linked list with the layout size
			size
		};
		segments.push(new_mem_chunk(new_seg_size));
		
		// Get the pointer to the thing we just allocated.
		// SAFETY:
		// segments is guaranteed to have a back because we just pushed stuff to it.
		// This ptr is non-null and aligned in memory.
		// ArenaAlloc has a phantom lifetime which invalidates all references before de-allocating the segments.
		let new_alloc_ptr = unsafe {
			let ptr = segments
				.last_mut()
				.unwrap_unchecked()
				.as_mut_ptr()
				.add(self.offset.get());
			let ptr = NonNull::new_unchecked(ptr).cast();
			NonNull::slice_from_raw_parts(ptr, size)
		};
		self.offset.set(self.offset.get() + size);
		Ok(new_alloc_ptr)
	}

	/// Memory cannot be deallocated individually from an arena. This function is a no-op.
	unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}

	/// Shrinking is not supported in Arenas because individual objects are not deallocated.
	/// Instead, this function asserts that the [new_layout]'s size is less than or equal to that of [old_layout].
	/// Then it returns [ptr] cast as a slice pointer with the size of [old_layout].
	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		debug_assert!(
			new_layout.size() <= old_layout.size(),
			"`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
		);
		Ok(NonNull::slice_from_raw_parts(ptr, old_layout.size()))
	}
}