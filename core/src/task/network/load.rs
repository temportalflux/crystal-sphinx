use super::Instruction;
use crate::app;
use engine::{
	network::mode,
	task::{ArctexState, ScheduledTask, Semaphore},
};
use std::{
	pin::Pin,
	sync::{Arc, RwLock},
	task::{Context, Poll},
};

pub struct Load {
	app_state: Arc<RwLock<app::state::Machine>>,
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
	pub fn load_dedicated_server(app_state: &Arc<RwLock<app::state::Machine>>) {
		Self::new(app_state.clone())
			.instruct(Instruction {
				name: "tmp".to_owned(),
				mode: mode::Kind::Server.into(),
			})
			.send_to(engine::task::sender());
	}

	pub fn add_state_listener(app_state: &Arc<RwLock<app::state::Machine>>) {
		use app::state::{State::*, Transition::*, *};
		let app_state_for_loader = app_state.clone();
		app_state.write().unwrap().add_callback(
			OperationKey(None, Some(Enter), Some(LoadingWorld)),
			move |operation| {
				let instruction = operation
					.data()
					.as_ref()
					.unwrap()
					.downcast_ref::<Instruction>()
					.unwrap()
					.clone();
				Self::new(app_state_for_loader.clone())
					.instruct(instruction)
					.send_to(engine::task::sender());
			},
		);
	}

	pub fn new(app_state: Arc<RwLock<app::state::Machine>>) -> Self {
		Self {
			app_state,
			state: ArctexState::default(),
		}
	}

	pub fn instruct(self, instruction: Instruction) -> Self {
		log::warn!(target: "world-loader", "Loading world at \"{}\"", instruction.name);

		let thread_state = self.state.clone();
		let thread_app_state = self.app_state.clone();
		std::thread::spawn(move || {
			use engine::{network::Network, task};

			// TODO: Kick off a loading task, once data is saved to disk
			std::thread::sleep(std::time::Duration::from_secs(3));

			let mut semaphores: Vec<Semaphore> = vec![];

			let _ = crate::network::create(&thread_app_state, instruction.mode).spawn();
			if Network::local_data().is_server() {
				use crate::server::Server;
				if let Ok(mut server) = Server::load(&instruction.name) {
					let world_loading_semaphore = server.start_loading_world();
					semaphores.push(world_loading_semaphore);
					if let Ok(mut guard) = Server::write() {
						(*guard) = Some(server);
					}
				}
			}

			task::wait_for_all(&mut semaphores, std::time::Duration::from_millis(100));
			thread_state.lock().unwrap().mark_complete();
		});
		self
	}
}

impl Drop for Load {
	fn drop(&mut self) {
		use app::state::State::InGame;
		self.app_state.write().unwrap().transition_to(InGame, None);
	}
}
