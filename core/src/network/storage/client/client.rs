use std::sync::{Arc, RwLock};

pub type ArcLockClient = Arc<RwLock<Client>>;
/// Container class for all client data which is present when a user is connected to a game server.
#[derive(Default)]
pub struct Client {
	entity_id: Option<hecs::Entity>,
}

impl Client {
	pub fn set_entity_id(&mut self, entity: hecs::Entity) {
		// TODO:
		// if the entity is already in the world, then add the client_only() components to it
		// if the entity is not in the world, when it gets replicated, append the client_only() components on construction
		self.entity_id = Some(entity);
	}
}
