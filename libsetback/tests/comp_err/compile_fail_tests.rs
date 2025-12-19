#![feature(allocator_api)]

use libsetback::Arena;

fn ref_lifetime() {
	let arena = Arena::new();
	let one = arena.alloc(1);
	drop(arena);
	assert_eq!(*one, 1);//borrow after move
}

fn box_lifetime() {
	let arena = Arena::new();
	let one = Box::new_in(1, arena.allocator());
	drop(arena);
	assert_eq!(*one, 1);//borrow after move
}

fn use_after_clear() {
	let arena = Arena::new();
	let one = arena.alloc(1);
	arena.clear();
	assert_eq!(*one, 1);//borrow after move
}

fn main() {
	ref_lifetime();
	box_lifetime();
	use_after_clear();
}