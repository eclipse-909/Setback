use {
	crate::SmallVec,
	std::sync::Arc,
	tokio::sync::Mutex,
};

pub struct FreeList<T, const CAPACITY: usize> {
	items: Mutex<SmallVec<T, CAPACITY>>,
}

impl<T, const CAPACITY: usize> FreeList<T, CAPACITY> {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			items: Mutex::new(SmallVec::new()),
		})
	}

	pub async fn take(self: &Self) -> T
	where
		T: Default,
	{
		let mut items = self.items.lock().await;
		if items.len() == 0 {
			T::default()
		} else {
			items.pop()
		}
	}

	pub async fn give(self: &Self, item: T) {
		let mut items = self.items.lock().await;
		if CAPACITY > items.len() as usize {
			items.push(item);
		}
	}
}
