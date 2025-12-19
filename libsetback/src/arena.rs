use std::{
	alloc::{AllocError, Allocator, Layout, System},
	cell::Cell,
	marker::PhantomData,
	ptr::{self, NonNull},
};

const PAGE_SIZE: usize = 4096;

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
			allocator: ArenaAlloc::new(),
		}
	}

	/// Creates an empty [Arena] with an initial capacity of at least the given amount.
	#[inline(always)]
	pub fn new_with_capacity(bytes: usize) -> Self {
		Self {
			allocator: ArenaAlloc::new_with_capacity(bytes),
		}
	}

	/// Allocates at least enough space for object and moves object into that space.
	/// Returns a reference to object whose lifetime is the same as the arena.
	/// # Panic
	/// Panics if the underlying heap allocation fails (if your system runs out of memory or something similar).
	#[allow(clippy::mut_from_ref)]
	pub fn alloc<T: Sized>(&self, object: T) -> &mut T {
		unsafe {
			let mut p = self.allocator.allocate(Layout::new::<T>()).unwrap().cast();
			p.write(object);
			p.as_mut()
		}
	}

	/// Clears the arena, calling drop on all items allocated with it.
	#[inline(always)]
	pub fn clear(&mut self) {
		_ = std::mem::replace(&mut self.allocator, ArenaAlloc::new());
	}

	/// Clears the arena, calling drop on all items allocated with it.
	/// Gives the arena a new capacity of at least the given amount.
	#[inline]
	pub fn clear_with_capacity(&mut self, bytes: usize) {
		_ = std::mem::replace(&mut self.allocator, ArenaAlloc::new_with_capacity(bytes));
	}

	///Returns a reference to the allocator used by the arena. This way you can use the arena for other things, like with [Vec::new_in].
	#[inline(always)]
	pub const fn allocator(&self) -> &ArenaAlloc<'arena> {
		&self.allocator
	}
}

/// The allocator used for [Arena]. This struct is wrapped by [Arena] to hide implementation details and unnecessary functions.
pub struct ArenaAlloc<'arena> {
	head: Cell<Option<NonNull<ArenaPage>>>,
	tail: Cell<Option<NonNull<ArenaPage>>>,
	offset: Cell<usize>,
	cap: usize,
	_phantom: PhantomData<&'arena ()>,
}

impl ArenaAlloc<'_> {
	#[inline(always)]
	const fn new() -> Self {
		Self {
			head: Cell::new(None),
			tail: Cell::new(None),
			offset: Cell::new(0),
			cap: 0,
			_phantom: PhantomData,
		}
	}
	#[inline]
	fn new_with_capacity(cap: usize) -> Self {
		let head = Cell::new(Some(ArenaPage::new_with_capacity(cap)));
		Self {
			head: head.clone(),
			tail: head,
			offset: Cell::new(0),
			cap,
			_phantom: PhantomData,
		}
	}
}

unsafe impl<'arena> Allocator for ArenaAlloc<'arena> {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let mut layout_size: usize = layout.size();
		let tail = if self.head.get().is_none() {
			//first segment in the linked list
			let new_page_ptr = ArenaPage::new_with_capacity(layout_size);
			self.tail.set(Some(new_page_ptr));
			self.head.set(Some(new_page_ptr));
			new_page_ptr
		} else {
			let tail = self.tail.get().unwrap();
			if self.offset.get() + layout_size >= unsafe { tail.read().mem_ptr.len() } {
				//Create a new segment and make that the tail
				//Double the size of the entire arena
				layout_size += self.cap;
				log::info!("Increasing arena size to {}", layout_size + self.cap);
				let new_page_ptr = ArenaPage::new_with_capacity(layout_size);
				unsafe {
					tail.read().next = Some(new_page_ptr);
				}
				self.tail.set(Some(new_page_ptr));
				self.offset.set(0);
				new_page_ptr
			} else {
				tail
			}
		};
		//Allocate the memory in the tail segment by adding the size to the index
		let ret_ptr: NonNull<[u8]> = NonNull::slice_from_raw_parts(
			unsafe { tail.read().mem_ptr.as_non_null_ptr().add(self.offset.get()) },
			layout_size,
		);
		self.offset.set(self.offset.get() + layout_size);
		Ok(ret_ptr)
	}

	///Memory cannot be deallocated individually from an arena. This function does nothing.
	#[inline(always)]
	unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}

	///Same as [Allocator::grow] but no memory is deallocated here.
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
		log::info!("Arena is growing to {} bytes", new_layout.size());
		let new_ptr: NonNull<[u8]> = self.allocate(new_layout)?;
		unsafe {
			ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.cast().as_ptr(), old_layout.size());
		}
		Ok(new_ptr)
	}

	///Same as [Allocator::grow_zeroed] but no memory is deallocated here.
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
		let new_ptr: NonNull<[u8]> = self.allocate_zeroed(new_layout)?;
		unsafe {
			ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.cast().as_ptr(), old_layout.size());
		}
		Ok(new_ptr)
	}

	///Shrinking is not supported in Arenas because individual objects are not deallocated.
	///Instead, this function asserts that the [new_layout]'s size is less than or equal to that of [old_layout].
	///Then it returns [ptr] cast as a slice pointer with the size of [old_layout].
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

impl<'arena> Drop for ArenaAlloc<'arena> {
	fn drop(&mut self) {
		let mut current = self.head.get();
		while let Some(curr) = current {
			unsafe {
				let next = curr.read().next;
				//deallocate the memory
				System.deallocate(
					curr.read().mem_ptr.as_non_null_ptr(),
					Layout::from_size_align(curr.read().mem_ptr.len(), 1).unwrap(),
				);
				//deallocate the page
				System.deallocate(curr.cast(), Layout::new::<ArenaPage>());
				current = next;
			}
		}
	}
}

struct ArenaPage {
	///Fat pointer to the heap-allocated memory.
	mem_ptr: NonNull<[u8]>,
	///The next page in the linked-list.
	next: Option<NonNull<Self>>,
}
impl ArenaPage {
	fn new_with_capacity(cap: usize) -> NonNull<Self> {
		//allocate the page itself
		let new_page_ptr: NonNull<Self> = System.allocate(Layout::new::<Self>()).unwrap().cast();
		unsafe {
			new_page_ptr.write(Self {
				//allocate memory for the page
				mem_ptr: System
					.allocate(
						Layout::from_size_align((cap / PAGE_SIZE + 1) * PAGE_SIZE, 1).unwrap(),
					)
					.unwrap(),
				next: None,
			})
		};
		new_page_ptr
	}
}
