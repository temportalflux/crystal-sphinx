use super::super::ArcLockDatabase;
use engine::{
	task::{ArctexState, ScheduledTask},
	utility::{spawn_thread, VoidResult},
};
use std::{
	pin::Pin,
	task::{Context, Poll},
};

pub struct Load {
	state: ArctexState,
}

impl ScheduledTask for Load {
	fn state(&self) -> &ArctexState {
		&self.state
	}
}
impl futures::future::Future for Load {
	type Output = ();
	fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
		self.poll_state(ctx)
	}
}

impl Load {
	pub fn new(world: &ArcLockDatabase) -> Self {
		let state = ArctexState::default();

		let thread_world = world.clone();
		let thread_state = state.clone();
		spawn_thread("world-loader", move || -> VoidResult {
			let mut world = thread_world.write().unwrap();

			// Ensure the world directory exists
			if !world.root_path.exists() {
				std::fs::create_dir_all(&world.root_path)?;
			}

			// Load settings from disk, if it exists
			let settings_path = world.settings_path();
			if settings_path.exists() {
				let raw = std::fs::read_to_string(&settings_path)?;
				world.settings = serde_json::from_str(&raw)?;
			}

			// Auto-save loaded settings to file
			{
				let json = serde_json::to_string_pretty(&world.settings)?;
				std::fs::write(&settings_path, json)?;
			}

			thread_state.lock().unwrap().mark_complete();
			Ok(())
		});

		Self { state }
	}
}
