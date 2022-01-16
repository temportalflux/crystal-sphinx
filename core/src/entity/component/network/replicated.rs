use crate::entity::component::Component;

/// Attached to entities to mark them (and any components which are [`Replicatable`](super::Replicatable)) for replication.
#[derive(Default)]
pub struct Replicated {}

impl Component for Replicated {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::network::Replicated"
	}

	fn display_name() -> &'static str {
		"Replicated"
	}
}
