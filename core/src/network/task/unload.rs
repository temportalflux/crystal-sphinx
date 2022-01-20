use crate::app;
use std::sync::{Arc, RwLock};

pub fn add_unloading_state_listener(app_state: &Arc<RwLock<app::state::Machine>>) {
	use app::state::{State::*, Transition::*, *};
	let app_state_for_loader = app_state.clone();
	app_state.write().unwrap().add_async_callback(
		OperationKey(None, Some(Enter), Some(Unloading)),
		move |_operation| {
			let async_state = app_state_for_loader.clone();
			async move {
				// TODO: Kick off a unloading task, once data is saved to disk
				std::thread::sleep(std::time::Duration::from_secs(3));

				if let Ok(mut app_state) = async_state.write() {
					app_state.transition_to(app::state::State::MainMenu, None);
				}

				Ok(())
			}
		},
	);
}
