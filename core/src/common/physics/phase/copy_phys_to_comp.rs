use crate::{common::physics::Context, entity};

/// Copies data from the physics simulation to the entity components'.
pub(in crate::common::physics) struct CopyPhysicsToComponents;
impl CopyPhysicsToComponents {
	pub fn execute(ctx: &mut Context, world: &mut entity::World) {
		profiling::scope!("copy physics -> components");
		// TODO: copy changes in physics system into ecs components
	}
}
