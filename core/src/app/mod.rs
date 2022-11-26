use std::sync::{Arc, RwLock};

pub mod state;

/// Creates a [`Storage`](state::storage::Storage) cache (a Arc-Mutex owned by one or more callbacks)
/// which runs the `fn_create` callback when the app enters a given state,
/// and destroys an object created by that callback when the app leaves the given state.
/// The data returned by `fn_create` effectively is bound to the lifetime of the provided state, even though that is not semantically clear.
pub fn store_during<T, F>(
	app_state: &Arc<RwLock<state::Machine>>,
	state: state::State,
	fn_create: F,
) where
	T: 'static + Send + Sync,
	F: (Fn() -> anyhow::Result<Option<T>>) + 'static + Send + Sync,
{
	use state::{
		storage::{Callback, Storage},
		OperationKey,
		Transition::*,
	};

	Storage::<T>::default()
		.create_when(OperationKey(None, Some(Enter), Some(state)))
		.destroy_when(OperationKey(Some(state), Some(Exit), None))
		.with_callback(Callback::recurring(fn_create))
		.build(&app_state);
}

pub fn store_during_once<T, F>(
	app_state: &Arc<RwLock<state::Machine>>,
	state: state::State,
	fn_create: F,
) where
	T: 'static + Send + Sync,
	F: (FnOnce() -> anyhow::Result<Option<T>>) + 'static + Send + Sync,
{
	use state::{
		storage::{Callback, Storage},
		OperationKey,
		Transition::*,
	};

	// TODO: This should be a one-off, but because the callbacks are stored,
	// it is actually called each time the game enters the state.
	// The function should be FnOnce and the storage should be discarded on exit.
	Storage::<T>::default()
		.create_when(OperationKey(None, Some(Enter), Some(state)))
		.destroy_when(OperationKey(Some(state), Some(Exit), None))
		.with_callback(Callback::once(fn_create))
		.build(&app_state);
}
