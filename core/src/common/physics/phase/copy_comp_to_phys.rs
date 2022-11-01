use crate::{common::physics::Context, entity};

/// Copies data from the entity components' to the physics simulation.
pub(in crate::common::physics) struct CopyComponentsToPhysics;
impl CopyComponentsToPhysics {
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("copy components -> physics");
		// TODO: copy changes in ecs components into physics system
	}
}
