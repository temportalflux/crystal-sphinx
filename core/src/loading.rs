use crate::app;
use engine::network::mode;
use futures::future::Future;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct Instruction {
	pub name: String,
	pub seed: Option<String>,
	pub mode: mode::Set,
}

struct State {
	is_complete: bool,
	waker: Option<std::task::Waker>,
}

pub struct TaskLoadWorld {
	app_state: Arc<RwLock<app::state::Machine>>,
	/// Indicates if the task is complete and how to tell the futures package when the task wakes up.
	state: Arc<Mutex<State>>,
}

impl Future for TaskLoadWorld {
	type Output = ();
	fn poll(
		self: std::pin::Pin<&mut Self>,
		ctx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		use std::task::Poll;
		let mut state = self.state.lock().unwrap();
		if !state.is_complete {
			state.waker = Some(ctx.waker().clone());
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	}
}

impl TaskLoadWorld {
	pub fn load_dedicated_server(app_state: &Arc<RwLock<app::state::Machine>>) {
		Self::new(app_state.clone())
			.instruct(Instruction {
				name: "tmp".to_owned(),
				seed: None,
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
		let state = Arc::new(Mutex::new(State {
			is_complete: false,
			waker: None,
		}));

		Self { app_state, state }
	}

	pub fn instruct(self, instruction: Instruction) -> Self {
		log::warn!(target: "world-loader", "Loading world at \"{}\" with seed {:?}", instruction.name, instruction.seed);

		let thread_state = self.state.clone();
		let thread_app_state = self.app_state.clone();
		std::thread::spawn(move || {
			use engine::network::Network;

			// TODO: Kick off a loading task, once data is saved to disk
			std::thread::sleep(std::time::Duration::from_secs(3));

			let _ = crate::network::create(&thread_app_state, instruction.mode).spawn();
			if Network::local_data().is_server() {
				use crate::server::Server;
				if let Ok(mut server) = Server::load(&instruction.name) {
					server.start_loading_world(&instruction.seed);
					if let Ok(mut guard) = Server::write() {
						(*guard) = Some(server);
					}
				}
			}

			let mut state = thread_state.lock().unwrap();
			state.is_complete = true;
			if let Some(waker) = state.waker.take() {
				waker.wake();
			}
		});
		self
	}

	/// Sends the task to the engine task management,
	/// where it will run until the operation is complete,
	/// and then be dropped (thereby dropping all of its contents).
	pub fn send_to(self, spawner: &Arc<engine::task::Sender>) {
		spawner.spawn(self)
	}
}

impl Drop for TaskLoadWorld {
	fn drop(&mut self) {
		use app::state::State::InGame;
		self.app_state.write().unwrap().transition_to(InGame, None);
	}
}

pub struct TaskUnloadWorld {
	app_state: Arc<RwLock<app::state::Machine>>,
	/// Indicates if the task is complete and how to tell the futures package when the task wakes up.
	state: Arc<Mutex<State>>,
}

impl Future for TaskUnloadWorld {
	type Output = ();
	fn poll(
		self: std::pin::Pin<&mut Self>,
		ctx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		use std::task::Poll;
		let mut state = self.state.lock().unwrap();
		if !state.is_complete {
			state.waker = Some(ctx.waker().clone());
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	}
}

impl TaskUnloadWorld {
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
		let state = Arc::new(Mutex::new(State {
			is_complete: false,
			waker: None,
		}));

		let thread_state = state.clone();
		std::thread::spawn(move || {
			// TODO: Kick off a unloading task, once data is saved to disk
			std::thread::sleep(std::time::Duration::from_secs(3));

			let mut state = thread_state.lock().unwrap();
			state.is_complete = true;
			if let Some(waker) = state.waker.take() {
				waker.wake();
			}
		});

		Self { app_state, state }
	}

	/// Sends the task to the engine task management,
	/// where it will run until the operation is complete,
	/// and then be dropped (thereby dropping all of its contents).
	pub fn send_to(self, spawner: &Arc<engine::task::Sender>) {
		spawner.spawn(self)
	}
}

impl Drop for TaskUnloadWorld {
	fn drop(&mut self) {
		use app::state::State::MainMenu;
		self.app_state
			.write()
			.unwrap()
			.transition_to(MainMenu, None);
	}
}
