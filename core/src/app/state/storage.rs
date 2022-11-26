use crate::app::state::{self, ArcLockMachine, OperationKey};
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub enum Callback<T> {
	Recurring(Box<dyn Fn() -> Result<Option<T>> + 'static + Send + Sync>),
	Once(Box<dyn FnOnce() -> Result<Option<T>> + 'static + Send + Sync>),
}
impl<T> Callback<T> {
	pub fn recurring<F>(callback: F) -> Self
	where
		F: Fn() -> Result<Option<T>> + 'static + Send + Sync,
	{
		Self::Recurring(Box::new(callback))
	}

	pub fn once<F>(callback: F) -> Self
	where
		F: FnOnce() -> Result<Option<T>> + 'static + Send + Sync,
	{
		Self::Once(Box::new(callback))
	}
}

pub struct Storage<T> {
	create: Option<OperationKey>,
	destroy: Option<OperationKey>,
	creator: Option<Callback<T>>,
	_phantom: std::marker::PhantomData<T>,
}
impl<T> Default for Storage<T> {
	fn default() -> Self {
		Self {
			create: None,
			destroy: None,
			creator: None,
			_phantom: Default::default(),
		}
	}
}
impl<T> Storage<T>
where
	T: 'static + Send + Sync,
{
	pub fn create_when(mut self, key: OperationKey) -> Self {
		self.create = Some(key);
		self
	}

	pub fn destroy_when(mut self, key: OperationKey) -> Self {
		self.destroy = Some(key);
		self
	}

	pub fn with_callback(mut self, callback: Callback<T>) -> Self {
		self.creator = Some(callback);
		self
	}

	pub fn build(self, app_state: &ArcLockMachine) {
		let creator = self.creator.unwrap();
		let mut app_state = app_state.write().unwrap();
		let storage: Arc<Mutex<Option<T>>> = Default::default();

		if let Some(operation_key) = self.destroy {
			let storage_in_fn = storage.clone();
			let callback = match &creator {
				Callback::Recurring(_) => state::Callback::recurring(move |_| {
					let mut storage = storage_in_fn.lock().unwrap();
					*storage = None;
				}),
				Callback::Once(_) => state::Callback::once(move |_| {
					let mut storage = storage_in_fn.lock().unwrap();
					*storage = None;
				}),
			};
			app_state.insert_callback(operation_key, callback);
		}

		if let Some(operation_key) = self.create {
			let storage_in_fn = storage.clone();
			let callback = match creator {
				Callback::Recurring(creator) => {
					state::Callback::recurring(move |_| match creator() {
						Ok(item) => {
							let mut storage = storage_in_fn.lock().unwrap();
							*storage = item;
						}
						Err(err) => {
							log::error!(target: "storage", "{:?}", err);
						}
					})
				}
				Callback::Once(creator) => state::Callback::once(move |_| match creator() {
					Ok(item) => {
						let mut storage = storage_in_fn.lock().unwrap();
						*storage = item;
					}
					Err(err) => {
						log::error!(target: "storage", "{:?}", err);
					}
				}),
			};
			app_state.insert_callback(operation_key, callback);
		}
	}
}
