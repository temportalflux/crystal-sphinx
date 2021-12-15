use super::{ArcLockMachine, OperationKey};
use std::sync::{Arc, Mutex};

pub enum Event {
	Create,
	Destroy,
}

pub struct Storage<T> {
	events: Vec<(OperationKey, Event)>,
	_phantom: std::marker::PhantomData<T>,
}
impl<T> Default for Storage<T> {
	fn default() -> Self {
		Self {
			events: Vec::new(),
			_phantom: Default::default(),
		}
	}
}

impl<T> Storage<T>
where
	T: 'static + Send + Sync,
{
	pub fn with_event(mut self, event: Event, key: OperationKey) -> Self {
		self.events.push((key, event));
		self
	}

	pub fn create_callbacks<F>(self, app_state: &ArcLockMachine, create_callback: F)
	where
		F: Fn() -> Option<T> + 'static + Send + Sync,
	{
		let storage: Arc<Mutex<Option<T>>> = Default::default();
		let creator = Arc::new(create_callback);

		let mut app_state = app_state.write().unwrap();
		for (operation_key, event) in self.events.into_iter() {
			let callback_storage = storage.clone();
			match event {
				Event::Create => {
					let callback_creator = creator.clone();
					app_state.add_callback(operation_key, move |_operation| {
						let mut storage = callback_storage.lock().unwrap();
						*storage = callback_creator();
					});
				}
				Event::Destroy => {
					app_state.add_callback(operation_key, move |_operation| {
						let mut storage = callback_storage.lock().unwrap();
						*storage = None;
					});
				}
			}
		}
	}
}
