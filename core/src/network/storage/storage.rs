use super::server::{ArcLockServer, Server};
use std::sync::{Arc, RwLock};

pub type ArcLockStorage = Arc<RwLock<Storage>>;
#[derive(Default)]
pub struct Storage {
	_client: Option<()>,
	server: Option<ArcLockServer>,
}

impl Storage {
	pub fn set_server(&mut self, server: Server) {
		self.server = Some(Arc::new(RwLock::new(server)));
	}

	pub fn server(&self) -> Option<&ArcLockServer> {
		self.server.as_ref()
	}

	pub fn start_loading(&self) {
		if let Some(arc_server) = self.server.as_ref() {
			if let Ok(mut server) = arc_server.write() {
				server.start_loading_world();
			}
		}
	}
}
