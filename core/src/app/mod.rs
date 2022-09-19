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
		storage::{Event::*, Storage},
		Transition::*,
		*,
	};

	Storage::<T>::default()
		.with_event(Create, OperationKey(None, Some(Enter), Some(state)))
		.with_event(Destroy, OperationKey(Some(state), Some(Exit), None))
		.create_callbacks(&app_state, fn_create);
}
