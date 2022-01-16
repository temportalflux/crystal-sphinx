use crate::entity::component::Component;

/// Attached to entities to mark them (and any components which are [`Replicatable`](super::Replicatable)) for replication.
/// This component itself is NOT REPLICATED, but it is created on clients for entities which are replicated.
/// Creating on the fly when it is received by clients saves a tiny bit of packet bandwidth.
pub struct Replicated {
	server_id: Option<hecs::Entity>,
}

impl Replicated {
	pub fn new_server() -> Self {
		Self { server_id: None }
	}

	pub fn new_client(server_id: hecs::Entity) -> Self {
		Self {
			server_id: Some(server_id),
		}
	}

	/// Returns a non-None id if called on the client.
	/// This will NOT return the id of the owning entity when called from a server.
	pub fn get_id_on_server(&self) -> Option<&hecs::Entity> {
		self.server_id.as_ref()
	}
}

impl Component for Replicated {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::network::Replicated"
	}

	fn display_name() -> &'static str {
		"Replicated"
	}
}
