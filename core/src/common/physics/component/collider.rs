use crate::entity::component::{binary, debug, network, Component, Registration};
use engine::channels::mpsc;
use enumset::{EnumSet, EnumSetType};
use rapier3d::prelude::{ActiveCollisionTypes, Group, InteractionGroups, SharedShape};
use serde::{Deserialize, Serialize};

/// Component-flag indicating if an entity has an equivalent collider in the physics system.
/// Created during the [`AddPhysicsObjects`] phase of [`Physics::update`] for any entities with a [`Collider`] component.
pub struct ColliderHandle {
	pub(in crate::common::physics) handle: rapier3d::prelude::ColliderHandle,
	pub(in crate::common::physics) on_drop: mpsc::Sender<rapier3d::prelude::ColliderHandle>,
}
impl Drop for ColliderHandle {
	fn drop(&mut self) {
		let _ = self.on_drop.send(self.handle);
	}
}
impl Component for ColliderHandle {
	fn unique_id() -> &'static str {
		"crystal_sphinx::common::physics::component::ColliderHandle"
	}

	fn display_name() -> &'static str {
		"ColliderHandle"
	}
}
impl ColliderHandle {
	pub fn inner(&self) -> &rapier3d::prelude::ColliderHandle {
		&self.handle
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Collider {
	shape: SharedShape,
	/// All colliders are solid by default, they represent a geometric shape that can have contact points
	/// with other colliders to generate contact forces to prevent objects from penetrating-each-others.
	///
	/// Sensor colliders on the other end don't generate contacts: they only generate intersection events
	/// when one sensor collider and another collider start/stop touching.
	/// Sensor colliders are generally used to detect when something enters an area.
	///
	/// https://www.rapier.rs/docs/user_guides/rust/colliders#collider-type
	sensor: bool,
	/// Indicates what other colliders and interact with this collider.
	///
	/// https://www.rapier.rs/docs/user_guides/rust/colliders#collision-groups-and-solver-groups
	pub(in crate::common::physics) interaction_groups: InteractionGroups,
	/// Determines if/how a collider attached to a rigid body interacts with this collider.
	///
	/// https://www.rapier.rs/docs/user_guides/rust/colliders#active-collision-types
	collision_types: ActiveCollisionTypes,
	/// How "bouncy" is the collider.
	restitution: f32,
}

impl Collider {
	pub fn new(shape: SharedShape) -> Self {
		Self {
			shape,
			sensor: false,
			interaction_groups: InteractionGroups::default(),
			collision_types: ActiveCollisionTypes::default(),
			restitution: 0.0,
		}
	}

	pub fn shape(&self) -> &SharedShape {
		&self.shape
	}

	/// Marks the collider as sensor-only.
	/// It will not actually generate forces with other colliders.
	pub fn with_sensor(mut self) -> Self {
		self.set_is_sensor(true);
		self
	}

	pub fn set_is_sensor(&mut self, only_sense_collisions: bool) {
		self.sensor = only_sense_collisions;
	}

	pub fn is_sensor(&self) -> bool {
		self.sensor
	}

	pub fn with_restitution(mut self, bounciness: f32) -> Self {
		self.set_restitution(bounciness);
		self
	}

	pub fn set_restitution(&mut self, bounciness: f32) {
		self.restitution = bounciness;
	}

	pub fn restitution(&self) -> f32 {
		self.restitution
	}

	/// The default for each collider is that it is present in ALL collision detection & interaction groups.
	/// In order to enable users to specify specific groups, this function will clear /both/ groups.
	pub fn without_any_collision_groups(mut self) -> Self {
		self.interaction_groups = InteractionGroups::none();
		self
	}

	pub fn with_collision_detection_group<T: EnumSetType>(mut self, group: EnumSet<T>) -> Self {
		self.add_collision_detection_group(group);
		self
	}

	pub fn without_collision_detection_group<T: EnumSetType>(mut self, group: EnumSet<T>) -> Self {
		self.remove_collision_detection_group(group);
		self
	}

	pub fn add_collision_detection_group<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups
			.memberships
			.insert(Group::from_bits_truncate(group.as_u32()));
	}

	pub fn remove_collision_detection_group<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups
			.memberships
			.remove(Group::from_bits_truncate(group.as_u32()));
	}

	pub fn set_collision_detection_groups<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups.memberships = Group::from_bits_truncate(group.as_u32());
	}

	pub fn collision_detection_groups<T: EnumSetType>(&self) -> EnumSet<T> {
		EnumSet::from_u32(self.interaction_groups.memberships.bits())
	}

	pub fn with_collision_interaction_group<T: EnumSetType>(mut self, group: EnumSet<T>) -> Self {
		self.add_collision_interaction_group(group);
		self
	}

	pub fn without_collision_interaction_group<T: EnumSetType>(
		mut self,
		group: EnumSet<T>,
	) -> Self {
		self.add_collision_interaction_group(group);
		self
	}

	pub fn add_collision_interaction_group<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups
			.filter
			.insert(Group::from_bits_truncate(group.as_u32()));
	}

	pub fn remove_collision_interaction_group<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups
			.filter
			.remove(Group::from_bits_truncate(group.as_u32()));
	}

	pub fn set_collision_interaction_groups<T: EnumSetType>(&mut self, group: EnumSet<T>) {
		self.interaction_groups.filter = Group::from_bits_truncate(group.as_u32());
	}

	pub fn collision_interaction_groups<T: EnumSetType>(&self) -> EnumSet<T> {
		EnumSet::from_u32(self.interaction_groups.filter.bits())
	}

	pub fn without_any_collision_types(mut self) -> Self {
		self.set_collision_types(ActiveCollisionTypes::empty());
		self
	}

	pub fn with_collision_types(mut self, collider_types: ActiveCollisionTypes) -> Self {
		self.add_collision_types(collider_types);
		self
	}

	pub fn add_collision_types(&mut self, collider_types: ActiveCollisionTypes) {
		self.collision_types.insert(collider_types);
	}

	pub fn remove_collision_types(&mut self, collider_types: ActiveCollisionTypes) {
		self.collision_types.remove(collider_types);
	}

	pub fn set_collision_types(&mut self, collider_types: ActiveCollisionTypes) {
		self.collision_types = collider_types;
	}

	pub fn collision_types(&self) -> &ActiveCollisionTypes {
		&self.collision_types
	}
}

impl std::fmt::Display for Collider {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Collider(TBD)",)
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
		*self = replicated.clone();
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
	fn render(&self, ui: &mut egui::Ui) {
		ui.label("TBD");
	}
}
