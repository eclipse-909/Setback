use {
	crate::PAGE_SIZE,
	std::{
		alloc::{AllocError, Allocator, Layout},
		cell::{Cell, UnsafeCell},
		marker::PhantomData,
		mem::MaybeUninit,
		ops::Deref,
		ptr::{self, NonNull},
		collections::LinkedList,
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
	#[inline(always)]
	#[allow(clippy::new_without_default)]
	pub const fn new() -> Self {
		Self {
			allocator: ArenaAlloc {
				_phantom: PhantomData,
				segments: UnsafeCell::new(LinkedList::new()),
				offset: Cell::new(0),
				cap: 0,
			}
		}
	}

	/// Creates an empty [Arena] with an initial capacity of at least the given amount.
	#[inline(always)]
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
	#[allow(clippy::mut_from_ref)]
	pub fn alloc<T: Sized>(&self, object: T) -> &mut T {
		// SAFETY:
		// memory is initialized with object
		unsafe {
			let mut p = self.allocator.allocate(Layout::new::<T>()).unwrap().cast();
			p.write(object);
			p.as_mut()
		}
	}
	
	/// Invalidates all reference to this arena and de-allocates most memory.
	/// The last segment in the arena is reused to start the arena.
	pub fn clear(&mut self) {
		if let Some(tail) = self.allocator.segments.get_mut().pop_back() {
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
	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		&self.allocator
	}
}

/// Fat pointer to the heap-allocated memory.
type ArenaSegment = Box<[MaybeUninit<u8>]>;

#[inline(always)]
fn new_mem_chunk(cap: usize) -> ArenaSegment {
	Box::new_uninit_slice((cap / PAGE_SIZE + 1) * PAGE_SIZE)
}

/// The allocator used for [Arena]. This struct is wrapped by [Arena] to hide implementation details and unnecessary functions.
pub struct ArenaAlloc<'arena> {
	/// Make sure this is the first field so the lifetime ends before the memory is deallocated when the arena is dropped.
	_phantom: PhantomData<&'arena mut ()>,
	// Contains all the memory allocated by the arena in a linked list of memory chunks.
	segments: UnsafeCell<LinkedList<ArenaSegment>>,
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
		let new_seg_size = if let Some(tail) = segments.back_mut() && self.offset.get() + size > tail.len() {
			// create a new segment with the size of the entire arena
			let new_seg_size = size + self.cap;
			debug!("Increasing arena size to {}", new_seg_size + self.cap);
			self.offset.set(0);
			new_seg_size
		} else {
			// create the first segment in the linked list with the layout size
			size
		};
		segments.push_back(new_mem_chunk(new_seg_size));
		
		// Get the pointer to the thing we just allocated.
		// SAFETY:
		// segments is guaranteed to have a back because we just pushed stuff to it.
		// This ptr is non-null and aligned in memory.
		// ArenaAlloc has a phantom lifetime which invalidates all references before de-allocating the segments.
		let new_alloc_ptr = unsafe {
			NonNull::slice_from_raw_parts(
				NonNull::new_unchecked(
					segments
						.back_mut()
						.unwrap_unchecked()
						.as_mut_ptr()
						.add(self.offset.get()),
				)
				.cast(),
				size,
			)
		};
		self.offset.set(self.offset.get() + size);
		Ok(new_alloc_ptr)
	}

	/// Memory cannot be deallocated individually from an arena. This function is a no-op.
	#[inline(always)]
	unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}

	/// Same as [Allocator::grow] but no memory is deallocated here.
	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		debug_assert!(
			new_layout.size() >= old_layout.size(),
			"`new_layout.size()` must be greater than or equal to `old_layout.size()`"
		);
		debug!("Arena is growing to {} bytes", new_layout.size());
		let new_ptr = self.allocate(new_layout)?;
		// SAFETY:
		// see debug assertion above
		unsafe {
			ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.cast().as_ptr(), old_layout.size());
		}
		Ok(new_ptr)
	}

	/// Same as [Allocator::grow_zeroed] but no memory is deallocated here.
	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		debug_assert!(
			new_layout.size() >= old_layout.size(),
			"`new_layout.size()` must be greater than or equal to `old_layout.size()`"
		);
		let new_ptr = self.allocate_zeroed(new_layout)?;
		// SAFETY:
		// see debug assertion above
		unsafe {
			ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.cast().as_ptr(), old_layout.size());
		}
		Ok(new_ptr)
	}

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

#[cfg(test)]
mod tests {
	use {
		super::Arena,
		std::sync::atomic::{AtomicU8, Ordering},
	};

	#[test]
	fn test_compile_error() {
		trybuild::TestCases::new().compile_fail("tests/comp_err/arena.rs");
	}

	#[test]
	fn test_default() {
		let arena = Arena::new();
		let one = arena.alloc(1);
		let two = Box::new_in(2, &*arena);
		assert_eq!(*one + *two, 3);
	}

	#[test]
	fn test_many_allocations() {
		const ARR1_LEN: usize = 500;
		const ARR2_LEN: usize = 20;
		const ARR3_LEN: usize = 1024;
		let arena = Arena::new();
		let arr1 = arena.alloc([0; ARR1_LEN]);
		let arr2 = arena.alloc([0; ARR2_LEN]);
		let arr3 = arena.alloc([0; ARR3_LEN]);
		for (i, v) in arr1.iter_mut().enumerate() {
			*v = i;
		}
		for (i, v) in arr2.iter_mut().enumerate() {
			*v = i;
		}
		for (i, v) in arr3.iter_mut().enumerate() {
			*v = i;
		}
		for (i, v) in arr1.iter().enumerate() {
			assert_eq!(*v, i);
		}
		for (i, v) in arr2.iter().enumerate() {
			assert_eq!(*v, i)
		}
		for (i, v) in arr3.iter().enumerate() {
			assert_eq!(*v, i)
		}
		drop(arena);
	}

	#[test]
	fn test_vec_in_arena() {
		let arena = Arena::new();
		let mut vec1 = Vec::new_in(&*arena);
		let mut vec2 = Vec::new_in(&*arena);
		let mut vec3 = Vec::new_in(&*arena);
		for i in 0..32 {
			vec1.push(i);
		}
		for i in 0..32 {
			vec2.push(i);
		}
		for i in 0..32 {
			vec3.push(i);
		}
		vec1.shrink_to_fit();
		vec2.shrink_to_fit();
		vec3.shrink_to_fit();
		for i in 0..32 {
			assert_eq!(vec1[i], i);
			assert_eq!(vec2[i], i);
			assert_eq!(vec3[i], i);
		}
	}

	static DROP_COUNT: AtomicU8 = AtomicU8::new(0);
	struct DropStruct(usize);
	impl Drop for DropStruct {
		fn drop(&mut self) {
			DROP_COUNT.fetch_add(1, Ordering::Relaxed);
		}
	}

	#[test]
	fn test_dropping() {
		let arena = Arena::new();
		let drop_ref = arena.alloc(DropStruct(0));
		drop_ref.0 = 1;
		let mut drop_box = Box::new_in(DropStruct(0), &*arena);
		drop_box.0 = 1;
		drop(drop_box);
		drop(arena);
		assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 2);
	}

	#[test]
	fn test_alignment() {
		let arena = Arena::new();
		let bool0: *const _ = arena.alloc(true);
		let bool1: *const _ = arena.alloc(false);
		let int0: *const _ = arena.alloc(4u32);
		let bool2: *const _ = arena.alloc(true);
		// SAFETY:
		// The add function doesn't even say what is unsafe about it.
		unsafe {
			assert_eq!(bool0.add(1), bool1);
			assert_eq!(bool1.add(3), int0.cast());
			assert_eq!(int0.add(4).cast(), bool2);
		}
	}
}
