use super::{Directive, Instruction};
use crate::{
	app::{self, state::ArcLockMachine},
	entity::{ArcLockEntityWorld, self},
	network::{
		packet::Handshake,
		storage::{client::ArcLockClient, server::Server, ArcLockStorage},
	},
};
use engine::{
	network::{mode, LocalData},
	task::{ArctexState, ScheduledTask},
};
use std::{
	pin::Pin,
	sync::{Arc, Weak, RwLock},
	task::{Context, Poll},
};

pub struct Load {
	state: ArctexState,
	app_state: ArcLockMachine,
	storage: ArcLockStorage,
	entity_world: Weak<RwLock<entity::World>>,
	next_app_state: Option<app::state::State>,
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
	#[profiling::function]
	pub fn load_dedicated_server(
		app_state: &ArcLockMachine,
		storage: &ArcLockStorage,
		entity_world: &ArcLockEntityWorld,
	) {
		Self::new(app_state.clone(), storage.clone(), Arc::downgrade(&entity_world))
			.instruct(Instruction {
				mode: mode::Kind::Server.into(),
				port: LocalData::get_named_arg("host_port"),
				directive: Directive::LoadWorld("tmp".to_owned()),
			})
			.join(std::time::Duration::from_millis(100 * 1), None);
	}

	pub fn add_state_listener(
		app_state: &ArcLockMachine,
		storage: &ArcLockStorage,
		entity_world: &ArcLockEntityWorld,
	) {
		use app::state::{State::*, Transition::*, *};
		for state in [LoadingWorld, Connecting].iter() {
			let callback_app_state = app_state.clone();
			let callback_storage = storage.clone();
			let callback_entity_world = Arc::downgrade(&entity_world);
			app_state.write().unwrap().add_callback(
				OperationKey(None, Some(Enter), Some(*state)),
				move |operation| {
					profiling::scope!("load_client");
					let instruction = operation
						.data()
						.as_ref()
						.unwrap()
						.downcast_ref::<Instruction>()
						.unwrap()
						.clone();
					Self::new(
						callback_app_state.clone(),
						callback_storage.clone(),
						callback_entity_world.clone(),
					)
					.instruct(instruction)
					.send_to(engine::task::sender());
				},
			);
		}
	}

	fn new(
		app_state: ArcLockMachine,
		storage: ArcLockStorage,
		entity_world: Weak<RwLock<entity::World>>,
	) -> Self {
		Self {
			state: ArctexState::default(),
			app_state,
			storage,
			entity_world,
			next_app_state: None,
		}
	}

	#[profiling::function]
	pub fn instruct(mut self, instruction: Instruction) -> Self {
		self.next_app_state = instruction.get_next_app_state();

		let thread_state = self.state.clone();
		let thread_app_state = self.app_state.clone();
		let thread_storage = self.storage.clone();
		let thread_entity_world = self.entity_world.clone();
		std::thread::spawn(move || {
			profiling::register_thread!("load-world");
			if instruction.mode.contains(mode::Kind::Server) {
				let world_name = match &instruction.directive {
					Directive::LoadWorld(world_name) => world_name,
					_ => unimplemented!(),
				};
				if let Ok(server) = Server::load(&world_name) {
					if let Ok(mut storage) = thread_storage.write() {
						storage.set_server(server);
					}
				}
			}
			if instruction.mode.contains(mode::Kind::Client) {
				if let Ok(mut storage) = thread_storage.write() {
					storage.set_client(ArcLockClient::default());
				}
			}

			let entity_world = thread_entity_world.upgrade().unwrap();
			let socknet_port = instruction.port.unwrap_or(25565);
			let _ = crate::network::create(
				instruction.mode,
				&thread_app_state,
				&thread_storage,
				&entity_world,
			)
			.with_port(socknet_port)
			.spawn();
			if let Ok(storage) = thread_storage.read() {
				storage.start_loading();
			}

			if instruction.mode == mode::Kind::Client {}

			// Dedicated Client (mode == Client) needs to connect to the server.
			// Additionally... Integrated Client-Server (mode == Client + Server) should run
			// authentication against its save data.
			// We can't prevent a smart user from downloading save data and replacing the
			// user id's and public key with their own, but we can at least do a bare-bones
			// "your id and public keys match" authentication.
			// Really we want to run the auth flow here because it allows us to put
			// initialization for entities on the server in the handshake and
			// initialization for entities on the client in the replication packet,
			// running both for Integrated Client-Server/Client-on-top-of-Server.
			if instruction.mode.contains(mode::Kind::Client) {
				let url = if instruction.mode == mode::Kind::Client {
					match &instruction.directive {
						Directive::Connect(url) => url.clone(),
						_ => unimplemented!(),
					}
				} else {
					// for Cotos, the server url is the address we just initialized the network with
					format!("127.0.0.1:{}", socknet_port)
				};
				if let Err(err) = Handshake::connect_to_server(&url) {
					log::error!("{}", err);
				}
			}

			thread_state.lock().unwrap().mark_complete();
		});
		self
	}
}

impl Drop for Load {
	fn drop(&mut self) {
		if let Some(state) = self.next_app_state {
			self.app_state.write().unwrap().transition_to(state, None);
		}
	}
}
