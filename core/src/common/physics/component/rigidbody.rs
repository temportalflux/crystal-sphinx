use crate::entity::component::{binary, debug, network, Component, Registration};
use engine::channels::mpsc;
use nalgebra::Vector3;
use rapier3d::prelude::RigidBodyType;
use serde::{Deserialize, Serialize};

/// Component-flag indicating if an entity has an equivalent rigidbody in the physics system.
/// Created during the [`AddPhysicsObjects`] phase of [`Physics::update`] for any entities with a [`RigidBody`] component.
pub struct RigidBodyHandle {
	pub(in crate::common::physics) handle: rapier3d::prelude::RigidBodyHandle,
	pub(in crate::common::physics) on_drop: mpsc::Sender<rapier3d::prelude::RigidBodyHandle>,
}
impl Drop for RigidBodyHandle {
	fn drop(&mut self) {
		let _ = self.on_drop.send(self.handle);
	}
}
impl Component for RigidBodyHandle {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::RigidBodyHandle"
	}

	fn display_name() -> &'static str {
		"RigidBodyHandle"
	}
}
impl RigidBodyHandle {
	pub fn inner(&self) -> &rapier3d::prelude::RigidBodyHandle {
		&self.handle
	}
}

/// Component-flag indicating that the entity with a RigidBody (and handle) has moved in the most recent physics system update.
/// The updated transform is in the Position & Orientation structures.
pub struct RigidBodyIsActive;
impl Component for RigidBodyIsActive {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::RigidBodyIsActive"
	}

	fn display_name() -> &'static str {
		"RigidBodyIsActive"
	}
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct RigidBody {
	kind: RigidBodyType,
	/// The linear velocity of the body.
	/// When created, the body in the physics subsystem will be initialized with this value.
	/// In subsequent update/simulation steps, this value is written into the physics system
	/// (thereby overriding accumulative gravity), and copied back into the component when physics is done simulating.
	linear_velocity: Vector3<f32>,
	continuous_collision_detection: bool,
}

impl RigidBody {
	pub fn new(kind: RigidBodyType) -> Self {
		Self {
			kind,
			linear_velocity: Vector3::default(),
			continuous_collision_detection: false,
		}
	}

	pub fn kind(&self) -> RigidBodyType {
		self.kind
	}

	pub fn with_linear_velocity(mut self, velocity: Vector3<f32>) -> Self {
		self.set_linear_velocity(velocity);
		self
	}

	pub fn set_linear_velocity(&mut self, velocity: Vector3<f32>) {
		self.linear_velocity = velocity;
	}

	pub fn linear_velocity(&self) -> &Vector3<f32> {
		&self.linear_velocity
	}

	pub fn with_ccd_enabled(mut self, enabled: bool) -> Self {
		self.continuous_collision_detection = enabled;
		self
	}

	pub fn ccd_enabled(&self) -> bool {
		self.continuous_collision_detection
	}
}

impl std::fmt::Display for RigidBody {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"RigidBody({:?}, velocity=<{:.2}, {:.2}, {:.2}>)",
			self.kind, self.linear_velocity.x, self.linear_velocity.y, self.linear_velocity.z
		)
	}
}

impl Component for RigidBody {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::RigidBody"
	}

	fn display_name() -> &'static str {
		"RigidBody"
	}

	fn registration() -> Registration<Self> {
		Registration::<Self>::default()
			.with_ext(binary::Registration::from::<Self>())
			.with_ext(debug::Registration::from::<Self>())
			.with_ext(network::Registration::from::<Self>())
	}
}

impl network::Replicatable for RigidBody {
	fn on_replication(&mut self, replicated: &Self, _is_locally_owned: bool) {
		*self = *replicated;
	}
}

impl binary::Serializable for RigidBody {
	fn serialize(&self) -> anyhow::Result<Vec<u8>> {
		binary::serialize(&self)
	}
	fn deserialize(bytes: Vec<u8>) -> anyhow::Result<Self> {
		binary::deserialize::<Self>(&bytes)
	}
}

impl debug::EguiInformation for RigidBody {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("Kind: {:?}", self.kind));
		ui.label(format!(
			"Linear Velocity: <{:.2}, {:.2}, {:.2}>",
			self.linear_velocity.x, self.linear_velocity.y, self.linear_velocity.z
		));
	}
}
