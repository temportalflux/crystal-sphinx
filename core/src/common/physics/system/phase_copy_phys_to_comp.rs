use crate::{
	common::physics::{
		backend::{self, RigidBodySet, RigidBodyType},
		backend::{CollisionEvent, ContactForceEvent},
		component::{
			self, CollidingWith, CollisionEventFlags, Orientation, Position, RigidBody,
			RigidBodyIsActive,
		},
		system::{ObjectId, ObjectKind},
		State,
	},
	entity,
};
use engine::channels::mpsc;
use enumset::EnumSet;
use hecs::Query;
use multimap::MultiMap;
use std::collections::HashMap;

#[derive(Query)]
struct RigidBodyBundle<'c> {
	handle: &'c component::RigidBodyHandle,
	rigid_body: &'c mut RigidBody,
	position: &'c mut Position,
	orientation: Option<&'c mut Orientation>,
}

/// Copies data from the physics simulation to the entity components'.
pub(in crate::common::physics) struct CopyPhysicsToComponents {
	pub recv_collisions: mpsc::Receiver<CollisionEvent>,
	pub recv_contact_forces: mpsc::Receiver<ContactForceEvent>,
}
impl CopyPhysicsToComponents {
	pub fn execute(&mut self, ctx: &mut State, world: &mut entity::World) {
		profiling::scope!("copy physics -> components");
		self.copy_rigid_bodies(ctx, world);
		self.propogate_active_bodies(ctx, world);
		self.propogate_collisions(ctx, world);
	}

	#[profiling::function]
	fn copy_rigid_bodies(&self, ctx: &mut State, world: &mut entity::World) {
		for (_entity, components) in world.query_mut::<RigidBodyBundle>() {
			let RigidBodyBundle {
				handle,
				rigid_body,
				position,
				orientation,
			} = components;

			let source = ctx.rigid_bodies.get(*handle.inner()).unwrap();
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
			rigid_body.set_linear_velocity(*source.linvel());
		}
	}

	#[profiling::function]
	fn propogate_active_bodies(&self, ctx: &mut State, world: &mut entity::World) {
		let mut active_bodies = UpdateActiveBodies::new(world);
		active_bodies.get_bodies_marked_as_active();
		active_bodies.insert(&ctx.rigid_bodies, ctx.islands.active_dynamic_bodies());
		active_bodies.insert(&ctx.rigid_bodies, ctx.islands.active_kinematic_bodies());
		active_bodies.apply();
	}

	#[profiling::function]
	fn propogate_collisions(&mut self, ctx: &mut State, world: &mut entity::World) {
		let mut collisions = UpdateCollisions::new(world);
		while let Ok(collision_event) = self.recv_collisions.try_recv() {
			collisions.insert_collision(&ctx, collision_event);
		}
		while let Ok(contact_force_event) = self.recv_contact_forces.try_recv() {
			collisions.insert_contact(contact_force_event);
		}
		collisions.apply();
	}
}

struct UpdateActiveBodies<'world> {
	world: &'world mut entity::World,
	previously_active_bodies: HashMap<backend::RigidBodyHandle, hecs::Entity>,
	transaction: hecs::CommandBuffer,
}
impl<'world> UpdateActiveBodies<'world> {
	fn new(world: &'world mut entity::World) -> Self {
		Self {
			world,
			previously_active_bodies: HashMap::new(),
			transaction: hecs::CommandBuffer::new(),
		}
	}

	// Fetch the list of all entities which have moved (are awake) in the most recent update.
	#[profiling::function]
	fn get_bodies_marked_as_active(&mut self) {
		self.previously_active_bodies = self
			.world
			.query_mut::<&component::RigidBodyHandle>()
			.with::<&RigidBodyIsActive>()
			.into_iter()
			.map(|(entity, handle)| (*handle.inner(), entity))
			.collect();
	}

	#[profiling::function]
	fn insert(&mut self, rigid_bodies: &RigidBodySet, active_handles: &[backend::RigidBodyHandle]) {
		for handle in active_handles.iter() {
			// If the handle was previously active, then it is still active and should not be left in the list.
			// Otherwise, it was not in the list, then we should add a new component for that entity.
			if self.previously_active_bodies.remove(handle).is_none() {
				let rigid_body = rigid_bodies.get(*handle).unwrap();
				let obj_id = ObjectId::from(rigid_body.user_data);
				if let ObjectKind::Entity(entity) = obj_id.kind {
					if self.world.contains(entity) {
						self.transaction.insert_one(entity, RigidBodyIsActive);
					} else {
						// ERROR: entity doesn't exist in the world
					}
				} else {
					// ERROR: invalid entity data
				}
			}
		}
	}

	#[profiling::function]
	fn apply(mut self) {
		for (_handle, entity) in self.previously_active_bodies.into_iter() {
			self.transaction.remove_one::<RigidBodyIsActive>(entity);
		}
		self.transaction.run_on(self.world);
	}
}

struct UpdateCollisions<'world> {
	world: &'world mut entity::World,
	transaction: hecs::CommandBuffer,
	started_collisions: MultiMap<hecs::Entity, (ObjectId, EnumSet<CollisionEventFlags>)>,
	stopped_collisions: MultiMap<hecs::Entity, (ObjectId, EnumSet<CollisionEventFlags>)>,
}
impl<'world> UpdateCollisions<'world> {
	fn new(world: &'world mut entity::World) -> Self {
		Self {
			world,
			transaction: hecs::CommandBuffer::new(),
			started_collisions: MultiMap::new(),
			stopped_collisions: MultiMap::new(),
		}
	}

	#[profiling::function]
	fn insert_collision(&mut self, ctx: &State, event: CollisionEvent) {
		let (cache, id1, id2, flags) = match event {
			CollisionEvent::Started(handle1, handle2, flags) => {
				let id1 = self.get_object_for(ctx, handle1);
				let id2 = self.get_object_for(ctx, handle2);
				let Some((id1, id2)) = id1.zip(id2) else { return };
				(
					&mut self.started_collisions,
					id1,
					id2,
					CollisionEventFlags::from(flags),
				)
			}
			CollisionEvent::Stopped(handle1, handle2, flags) => {
				let id1 = self.get_object_for(ctx, handle1);
				let id2 = self.get_object_for(ctx, handle2);
				let Some((id1, id2)) = id1.zip(id2) else { return };
				(
					&mut self.stopped_collisions,
					id1,
					id2,
					CollisionEventFlags::from(flags),
				)
			}
		};
		if let ObjectKind::Entity(entity) = id1.kind {
			cache.insert(entity, (id2.clone(), flags));
		}
		if let ObjectKind::Entity(entity) = id2.kind {
			cache.insert(entity, (id1.clone(), flags));
		}
	}

	fn insert_contact(&mut self, _event: ContactForceEvent) {
		// TODO
	}

	#[profiling::function]
	fn apply(mut self) {
		for (entity, colliding_with) in self.world.query_mut::<&mut CollidingWith>().into_iter() {
			// Clear the started and stopped lists so we can detect which (if any) events were new this frame.
			colliding_with.clear_updates();
			// Add all of the new-collision events
			if let Some(collisions) = self.started_collisions.remove(&entity) {
				colliding_with.start_many(collisions);
			}
			// Add all of the old-collision events
			if let Some(collisions) = self.stopped_collisions.remove(&entity) {
				colliding_with.stop_many(collisions);
			}
			// If there are no new events (nothing inserted) in this update then we can safely destroy the component.
			if colliding_with.is_empty() && !colliding_with.has_new_events() {
				self.transaction.remove_one::<CollidingWith>(entity);
			}
		}

		// Process all of the new-collision events for entities which were not colliding in the last 2 updates.
		for (entity, collisions) in self.started_collisions.into_iter() {
			let mut colliding_with = CollidingWith::new();
			colliding_with.start_many(collisions);
			self.transaction.insert_one(entity, colliding_with);
		}

		self.transaction.run_on(self.world);
	}

	fn get_object_for(&self, ctx: &State, handle: backend::ColliderHandle) -> Option<ObjectId> {
		let collider = ctx.colliders.get(handle).unwrap();
		let obj_id = ObjectId::from(collider.user_data);
		if let ObjectKind::Entity(entity) = obj_id.kind {
			if !self.world.contains(entity) {
				// ERROR: entity doesn't exist in the world
				return None;
			}
		}
		Some(obj_id)
	}
}
