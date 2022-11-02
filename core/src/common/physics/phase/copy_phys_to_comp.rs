use crate::{
	common::physics::{
		component::{Orientation, Position, RigidBody, RigidBodyHandle},
		Context,
	},
	entity,
};
use hecs::Query;
use rapier3d::prelude::RigidBodyType;

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
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("copy physics -> components");
		Self::copy_rigid_bodies(ctx, world);
		Self::propogate_collisions(ctx, world);
	}

	#[profiling::function]
	fn copy_rigid_bodies(ctx: &mut Context, world: &mut entity::World) {
		for (entity, components) in world.query_mut::<RigidBodyBundle>() {
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
	fn propogate_collisions(ctx: &mut Context, world: &mut entity::World) {}
}
