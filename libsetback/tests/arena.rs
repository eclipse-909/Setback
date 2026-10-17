#[cfg(test)]
mod tests {
	use {
		setback::Arena,
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
		let two = arena.boxed(2);
		assert_eq!(*one + *two, 3);
	}
	
	#[test]
	fn test_many_allocations() {
		const ARR1_LEN: usize = 500;
		const ARR2_LEN: usize = 20;
		const ARR3_LEN: usize = 1024;
		let arena = Arena::new();
		let arr1 = arena.alloc_mut([0; ARR1_LEN]);
		let arr2 = arena.alloc_mut([0; ARR2_LEN]);
		let arr3 = arena.alloc_mut([0; ARR3_LEN]);
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
		let mut vec1 = arena.vec();
		let mut vec2 = arena.vec();
		let mut vec3 = arena.vec();
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
		let drop_ref = arena.alloc_mut(DropStruct(0));
		drop_ref.0 = 1;
		let mut drop_box = arena.boxed(DropStruct(0));
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