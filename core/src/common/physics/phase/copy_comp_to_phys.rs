use crate::{
	common::physics::{
		component::{Collider, ColliderHandle, Orientation, Position, RigidBody, RigidBodyHandle},
		Context,
	},
	entity,
};
use hecs::Query;
use nalgebra::Isometry3;
use rapier3d::prelude::RigidBodyType;

#[derive(Query)]
struct RigidBodyBundle<'c> {
	rigid_body: &'c RigidBody,
	handle: &'c RigidBodyHandle,
	position: &'c Position,
	orientation: Option<&'c Orientation>,
}

#[derive(Query)]
struct ColliderBundle<'c> {
	collider: &'c Collider,
	handle: &'c ColliderHandle,
	rigid_body: Option<&'c RigidBody>,
	position: Option<&'c Position>,
	orientation: Option<&'c Orientation>,
}

/// Copies data from the entity components' to the physics simulation.
pub(in crate::common::physics) struct CopyComponentsToPhysics;
impl CopyComponentsToPhysics {
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("copy components -> physics");
		Self::copy_rigid_bodies(ctx, world);
		Self::copy_colliders(ctx, world);
	}

	#[profiling::function]
	fn copy_rigid_bodies(ctx: &mut Context, world: &mut entity::World) {
		for (_entity, components) in world.query::<RigidBodyBundle>().iter() {
			let RigidBodyBundle {
				rigid_body,
				handle,
				position,
				orientation,
			} = components;
			let target = ctx.rigid_bodies.get_mut(handle.0).unwrap();
			target.set_body_type(rigid_body.kind());
			match rigid_body.kind() {
				// Kinematics are driven by game logic, so their isometries are directly copied into physics
				RigidBodyType::KinematicPositionBased => {
					target.set_next_kinematic_position(position.isometry(orientation));
				}
				RigidBodyType::KinematicVelocityBased => {
					if target.linvel() != rigid_body.linear_velocity() {
						target.set_linvel(*rigid_body.linear_velocity(), true);
					}
				}
				// Dynamic bodies are driven by physics, so only copy inputs like velocity.
				RigidBodyType::Dynamic => {
					if target.linvel() != rigid_body.linear_velocity() {
						target.set_linvel(*rigid_body.linear_velocity(), true);
					}
				}
				RigidBodyType::Fixed => {} // NO-OP
			}
		}
	}

	#[profiling::function]
	fn copy_colliders(ctx: &mut Context, world: &mut entity::World) {
		let mut colliders = ctx.colliders.write().unwrap();
		for (_entity, components) in world.query::<ColliderBundle>().iter() {
			let ColliderBundle {
				collider,
				handle,
				rigid_body,
				position,
				orientation,
			} = components;
			let target = colliders.get_mut(handle.0).unwrap();

			if rigid_body.is_none() {
				target.set_position(match (position, orientation) {
					(Some(position), orientation) => position.isometry(orientation),
					(None, Some(orientation)) => orientation.isometry(),
					_ => Isometry3::identity(),
				});
			}

			target.set_sensor(collider.is_sensor());
			target.set_shape(collider.shape().clone());
			target.set_collision_groups(collider.interaction_groups);
			target.set_restitution(collider.restitution());
			target.set_active_collision_types(*collider.collision_types());
		}
	}
}
