use crate::{
	common::physics::{
		component::{Collider, ColliderHandle, Orientation, Position, RigidBody, RigidBodyHandle},
		Context,
	},
	entity,
};
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
pub(in crate::common::physics) struct AddPhysicsObjects;
impl AddPhysicsObjects {
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("add-physics-objects");
		Self::add_rigid_bodies(ctx, world);
		Self::add_colliders(ctx, world);
	}

	#[profiling::function]
	fn add_rigid_bodies(ctx: &mut Context, world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::new();

		// Iterate over all entities which have the `RigidBody` component (and a position),
		// but do not yet have a rigid body physics handle.
		for (entity, components) in world.query::<RigidBodiesWithoutHandles>().iter() {
			let RigidBodyBundle {
				rigidbody,
				position,
				orientation,
			} = components;

			// Make a rigid body for the entity.
			let mut rigid_body = RigidBodyBuilder::new(rigidbody.kind())
				// enable us to fetch the entity id for a rigidbody, providing a two-way mapping.
				.user_data(entity.to_bits().get() as _)
				.position(position.isometry(orientation))
				.linvel(*rigidbody.linear_velocity())
				.build();
			rigid_body.recompute_mass_properties_from_colliders(&*ctx.colliders.read().unwrap());
			let handle = RigidBodyHandle(ctx.rigid_bodies.insert(rigid_body));
			transaction.insert_one(entity, handle);
		}
		// Commit the rigid body changes so the collider query can access them.
		// The transaction will be empty after this.
		transaction.run_on(world);
	}

	#[profiling::function]
	fn add_colliders(ctx: &mut Context, world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::new();

		for (entity, components) in world.query::<CollidersWithoutHandles>().iter() {
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

			let collider = ColliderBuilder::new(collider.shape().clone())
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
				Some(rigid_body_handle) => ctx.colliders.write().unwrap().insert_with_parent(
					collider,
					rigid_body_handle.0,
					&mut ctx.rigid_bodies,
				),
				None => ctx.colliders.write().unwrap().insert(collider),
			};

			transaction.insert_one(entity, ColliderHandle::from(handle));
		}
		// Commit the collider components.
		transaction.run_on(world);
	}
}
