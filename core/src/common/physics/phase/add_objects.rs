use crate::{common::physics::Context, entity};

/// Adds physics objects to the simulation when new entities are detected.
pub(in crate::common::physics) struct AddPhysicsObjects;
impl AddPhysicsObjects {
	#[profiling::function]
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("add-physics-objects");
		// TODO: Add rigidbody and collider objects to sets if the component exists but the handle does not
	}
}
