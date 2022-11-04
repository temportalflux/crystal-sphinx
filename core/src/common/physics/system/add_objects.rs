use crate::{
	common::physics::{
		component::{Collider, ColliderHandle, Orientation, Position, RigidBody, RigidBodyHandle},
		State,
	},
	entity,
};
use engine::channels::mpsc;
use hecs::Query;
use nalgebra::Isometry3;
use rapier3d::prelude::{ActiveEvents, ColliderBuilder, RigidBodyBuilder};

#[derive(Query)]
struct RigidBodyBundle<'c> {
	rigidbody: &'c RigidBody,
	position: &'c Position,
	orientation: Option<&'c Orientation>,
}
type RigidBodiesWithoutHandles<'c> = hecs::Without<RigidBodyBundle<'c>, &'c RigidBodyHandle>;

#[derive(Query)]
struct ColliderBundle<'c> {
	collider: &'c mut Collider,
	rigidbody: Option<&'c RigidBodyHandle>,
	position: Option<&'c Position>,
	orientation: Option<&'c Orientation>,
}
type CollidersWithoutHandles<'c> = hecs::Without<ColliderBundle<'c>, &'c ColliderHandle>;

/// Adds physics objects to the simulation when new entities are detected.
pub(in crate::common::physics) struct AddPhysicsObjects {
	collider_channel: mpsc::Pair<rapier3d::prelude::ColliderHandle>,
	rigid_body_channel: mpsc::Pair<rapier3d::prelude::RigidBodyHandle>,
}
impl AddPhysicsObjects {
	pub fn new() -> Self {
		Self {
			collider_channel: mpsc::unbounded(),
			rigid_body_channel: mpsc::unbounded(),
		}
	}

	pub fn execute(&mut self, ctx: &mut State, world: &mut entity::World) {
		profiling::scope!("add-physics-objects");
		self.remove_dropped(ctx);
		self.add_rigid_bodies(ctx, world);
		self.add_colliders(ctx, world);
	}

	#[profiling::function]
	fn remove_dropped(&mut self, ctx: &mut State) {
		// Colliders first because some colliders may be children of rigidbodies who were also dropped,
		// but others may be standalone colliders or children of RBs which are not dropped.
		while let Ok(handle) = self.collider_channel.1.try_recv() {
			profiling::scope!(&format!("drop-collider"));
			ctx.colliders.remove(
				handle,
				&mut ctx.islands,
				&mut ctx.rigid_bodies,
				/*wake up attached rigid body*/ true,
			);
		}
		while let Ok(handle) = self.rigid_body_channel.1.try_recv() {
			profiling::scope!(&format!("drop-rigidbody"));
			ctx.rigid_bodies.remove(
				handle,
				&mut ctx.islands,
				&mut ctx.colliders,
				&mut ctx.impulse_joints,
				&mut ctx.multibody_joints,
				/*remove attached colliders*/ false,
			);
		}
	}

	#[profiling::function]
	fn add_rigid_bodies(&self, ctx: &mut State, world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::new();

		// Iterate over all entities which have the `RigidBody` component (and a position),
		// but do not yet have a rigid body physics handle.
		for (entity, components) in world.query::<RigidBodiesWithoutHandles>().iter() {
			profiling::scope!(&format!("add-rigid_body:{entity:?}"));
			let RigidBodyBundle {
				rigidbody,
				position,
				orientation,
			} = components;

			// Make a rigid body for the entity.
			let rigid_body = RigidBodyBuilder::new(rigidbody.kind())
				// TODO: Use a custom bit-field (u128), where 1 bit identifies entity vs static block, and 64-bits identify the entity id
				// enable us to fetch the entity id for a rigidbody, providing a two-way mapping.
				.user_data(entity.to_bits().get() as _)
				.position(position.isometry(orientation))
				.linvel(*rigidbody.linear_velocity())
				.ccd_enabled(rigidbody.ccd_enabled())
				.gravity_scale(1.0)
				.build();
			let handle = RigidBodyHandle {
				handle: ctx.rigid_bodies.insert(rigid_body),
				on_drop: self.rigid_body_channel.0.clone(),
			};
			transaction.insert_one(entity, handle);
		}
		// Commit the rigid body changes so the collider query can access them.
		// The transaction will be empty after this.
		transaction.run_on(world);
	}

	#[profiling::function]
	fn add_colliders(&self, ctx: &mut State, world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::new();

		for (entity, components) in world.query::<CollidersWithoutHandles>().iter() {
			profiling::scope!(&format!("add-collider:{entity:?}"));
			let ColliderBundle {
				collider,
				rigidbody,
				position,
				orientation,
			} = components;

			// If the collider has no rigidbody, then the position and orientation components
			// need to be propogated to the collider builder.
			// Otherwise, a rigidbody exists and its position will be used for the collider.
			let isometry = match (rigidbody, position, orientation) {
				(None, Some(position), orientation) => position.isometry(orientation),
				(None, None, Some(orientation)) => orientation.isometry(),
				_ => Isometry3::identity(),
			};

			let target = ColliderBuilder::new(collider.shape().clone())
				// enable us to fetch the entity id for a collider, providing a two-way mapping.
				.user_data(entity.to_bits().get() as _)
				.position(isometry)
				.sensor(collider.is_sensor())
				.active_collision_types(*collider.collision_types())
				.active_events(ActiveEvents::all())
				.collision_groups(collider.interaction_groups)
				.restitution(collider.restitution())
				.build();

			let handle = match rigidbody {
				Some(rigid_body_handle) => ctx.colliders.insert_with_parent(
					target,
					*rigid_body_handle.inner(),
					&mut ctx.rigid_bodies,
				),
				None => ctx.colliders.insert(target),
			};

			transaction.insert_one(
				entity,
				ColliderHandle {
					handle,
					on_drop: self.collider_channel.0.clone(),
				},
			);
		}
		// Commit the collider components.
		transaction.run_on(world);
	}
}
