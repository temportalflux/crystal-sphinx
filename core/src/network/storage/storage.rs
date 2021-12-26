use super::{
	client::ArcLockClient,
	server::{ArcLockServer, Server},
};
use crate::{app::state::ArcLockMachine, entity::ArcLockEntityWorld};
use std::sync::{Arc, RwLock};

pub type ArcLockStorage = Arc<RwLock<Storage>>;
#[derive(Default)]
pub struct Storage {
	client: Option<ArcLockClient>,
	server: Option<ArcLockServer>,
}

impl Storage {
	pub fn new(app_state: &ArcLockMachine) -> ArcLockStorage {
		use crate::app::state::{State::*, Transition::*, *};

		let arclocked = Arc::new(RwLock::new(Self::default()));

		// Add callback to clear the client storage if the client disconnects
		{
			let callback_storage = arclocked.clone();
			app_state.write().unwrap().add_callback(
				OperationKey(None, Some(Enter), Some(Disconnecting)),
				move |_operation| {
					if let Ok(mut storage) = callback_storage.write() {
						storage.client = None;
					}
				},
			);
		}

		// Add callback to clear the server storage if the server unloads
		{
			let callback_storage = arclocked.clone();
			app_state.write().unwrap().add_callback(
				OperationKey(None, Some(Enter), Some(Unloading)),
				move |_operation| {
					if let Ok(mut storage) = callback_storage.write() {
						storage.server = None;
					}
				},
			);
		}

		arclocked
	}

	pub fn arclocked(self) -> ArcLockStorage {
		Arc::new(RwLock::new(self))
	}

	pub fn set_server(&mut self, server: Server) {
		self.server = Some(Arc::new(RwLock::new(server)));
	}

	pub fn server(&self) -> &Option<ArcLockServer> {
		&self.server
	}

	pub fn set_client(&mut self, client: ArcLockClient) {
		self.client = Some(client);
	}

	pub fn client(&self) -> &Option<ArcLockClient> {
		&self.client
	}

	pub fn start_loading(&self, entity_world: &ArcLockEntityWorld) {
		if let Some(arc_server) = self.server.as_ref() {
			if let Ok(mut server) = arc_server.write() {
				server.start_loading_world();
				server.initialize_systems(&entity_world);
			}
		}
	}
}
