use crate::{
	common::physics::{
		component::{Orientation, Position, RigidBody, RigidBodyHandle, RigidBodyIsActive},
		State,
	},
	entity,
};
use hecs::Query;
use rapier3d::prelude::{RigidBodyHandle as PhysicsRBHandle, RigidBodySet, RigidBodyType};
use std::collections::{HashMap, HashSet};

#[derive(Query)]
struct RigidBodyBundle<'c> {
	handle: &'c RigidBodyHandle,
	rigid_body: &'c mut RigidBody,
	position: &'c mut Position,
	orientation: Option<&'c mut Orientation>,
}

/// Copies data from the physics simulation to the entity components'.
pub(in crate::common::physics) struct CopyPhysicsToComponents;
impl CopyPhysicsToComponents {
	pub fn execute(ctx: &mut State, world: &mut entity::World) {
		profiling::scope!("copy physics -> components");
		Self::copy_rigid_bodies(ctx, world);
		Self::propogate_active_bodies(ctx, world);
		Self::propogate_collisions(ctx, world);
	}

	#[profiling::function]
	fn copy_rigid_bodies(ctx: &mut State, world: &mut entity::World) {
		for (_entity, components) in world.query_mut::<RigidBodyBundle>() {
			let RigidBodyBundle {
				handle,
				rigid_body,
				position,
				orientation,
			} = components;

			let source = ctx.rigid_bodies.get(handle.0).unwrap();
			match rigid_body.kind() {
				RigidBodyType::Dynamic | RigidBodyType::KinematicVelocityBased => {
					let isometry = source.position();
					position.set_translation(isometry.translation);
					if let Some(orientation) = orientation {
						orientation.set_rotation(isometry.rotation);
					}
				}
				_ => {}
			}
		}
	}

	#[profiling::function]
	fn propogate_active_bodies(ctx: &mut State, world: &mut entity::World) {
		let mut active_bodies = UpdateActiveBodies::new(world);
		active_bodies.get_bodies_marked_as_active();
		active_bodies.insert(&ctx.rigid_bodies, ctx.islands.active_dynamic_bodies());
		active_bodies.insert(&ctx.rigid_bodies, ctx.islands.active_kinematic_bodies());
		ctx.active_entites = active_bodies.apply();
	}

	#[profiling::function]
	fn propogate_collisions(ctx: &mut State, world: &mut entity::World) {
		// TODO: Receive events from physics systems, clear cached collisions on colliders, and update the cached list with the new collisions
	}
}

struct UpdateActiveBodies<'world> {
	world: &'world mut entity::World,
	active_entities: HashSet<hecs::Entity>,
	previously_active_bodies: HashMap<PhysicsRBHandle, hecs::Entity>,
	transaction: hecs::CommandBuffer,
}
impl<'world> UpdateActiveBodies<'world> {
	fn new(world: &'world mut entity::World) -> Self {
		Self {
			world,
			previously_active_bodies: HashMap::new(),
			active_entities: HashSet::new(),
			transaction: hecs::CommandBuffer::new(),
		}
	}

	// Fetch the list of all entities which have moved (are awake) in the most recent update.
	#[profiling::function]
	fn get_bodies_marked_as_active(&mut self) {
		self.previously_active_bodies = self
			.world
			.query::<&RigidBodyHandle>()
			.with::<&RigidBodyIsActive>()
			.iter()
			.map(|(entity, handle)| (handle.0, entity))
			.collect();
	}

	#[profiling::function]
	fn insert(&mut self, rigid_bodies: &RigidBodySet, active_handles: &[PhysicsRBHandle]) {
		for handle in active_handles.iter() {
			match self.previously_active_bodies.remove(handle) {
				// If the handle was previously active, then it is still active and should not be left in the list.
				Some(entity) => {
					self.active_entities.insert(entity);
				}
				// If it is not in the list, then we should add a new component for that entity.
				None => {
					let rigid_body = rigid_bodies.get(*handle).unwrap();
					if let Some(entity) = hecs::Entity::from_bits(rigid_body.user_data as u64) {
						if self.world.contains(entity) {
							self.transaction.insert_one(entity, RigidBodyIsActive);
							self.active_entities.insert(entity);
						} else {
							// ERROR: entity doesn't exist in the world
						}
					} else {
						// ERROR: invalid entity data
					}
				}
			}
		}
	}

	#[profiling::function]
	fn apply(mut self) -> HashSet<hecs::Entity> {
		for (_handle, entity) in self.previously_active_bodies.into_iter() {
			self.transaction.remove_one::<RigidBodyIsActive>(entity);
		}
		self.transaction.run_on(self.world);
		self.active_entities
	}
}
