use crate::app;
use engine::task::{ArctexState, ScheduledTask};
use std::{
	pin::Pin,
	sync::{Arc, RwLock},
	task::{Context, Poll},
};

pub struct Unload {
	app_state: Arc<RwLock<app::state::Machine>>,
	state: ArctexState,
}

impl ScheduledTask for Unload {
	fn state(&self) -> &ArctexState {
		&self.state
	}
}
impl futures::future::Future for Unload {
	type Output = ();
	fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
		self.poll_state(ctx)
	}
}

impl Unload {
	pub fn add_state_listener(app_state: &Arc<RwLock<app::state::Machine>>) {
		use app::state::{State::*, Transition::*, *};
		let app_state_for_loader = app_state.clone();
		app_state.write().unwrap().add_callback(
			OperationKey(None, Some(Enter), Some(Unloading)),
			move |_operation| {
				Self::new(app_state_for_loader.clone()).send_to(engine::task::sender());
			},
		);
	}

	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		let state = ArctexState::default();

		let thread_state = state.clone();
		std::thread::spawn(move || {
			// TODO: Kick off a unloading task, once data is saved to disk
			std::thread::sleep(std::time::Duration::from_secs(3));

			thread_state.lock().unwrap().mark_complete();
		});

		Self { app_state, state }
	}
}

impl Drop for Unload {
	fn drop(&mut self) {
		use app::state::State::MainMenu;
		self.app_state
			.write()
			.unwrap()
			.transition_to(MainMenu, None);
	}
}
