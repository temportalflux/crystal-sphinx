use crate::entity::component::{binary, debug, network, Component, Registration};
use serde::{Deserialize, Serialize};

/// Component-flag indicating if an entity has an equivalent collider in the physics system.
/// Created during the [`AddPhysicsObjects`] phase of [`Physics::update`] for any entities with a [`Collider`] component.
pub struct ColliderHandle(pub(in crate::common::physics) rapier3d::prelude::ColliderHandle);
impl Component for ColliderHandle {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::ColliderHandle"
	}

	fn display_name() -> &'static str {
		"ColliderHandle"
	}
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Collider {}

impl std::fmt::Display for Collider {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Collider()",)
	}
}

impl Component for Collider {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::Collider"
	}

	fn display_name() -> &'static str {
		"Collider"
	}

	fn registration() -> Registration<Self> {
		Registration::<Self>::default()
			.with_ext(binary::Registration::from::<Self>())
			.with_ext(debug::Registration::from::<Self>())
			.with_ext(network::Registration::from::<Self>())
	}
}

impl network::Replicatable for Collider {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = *replicated;
	}
}

impl binary::Serializable for Collider {
	fn serialize(&self) -> anyhow::Result<Vec<u8>> {
		binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> anyhow::Result<Self> {
		binary::deserialize::<Self>(&bytes)
	}
}

impl debug::EguiInformation for Collider {
	fn render(&self, ui: &mut egui::Ui) {}
}
