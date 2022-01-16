use crate::entity::{self, component, ArcLockEntityWorld};
use engine::EngineSystem;
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c mut component::physics::linear::Position,
	&'c component::physics::linear::Velocity,
)>;

pub struct Physics {
	world: Weak<RwLock<entity::World>>,
}

impl Physics {
	pub fn new(world: &ArcLockEntityWorld) -> Self {
		Self {
			world: Arc::downgrade(&world),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for Physics {
	fn update(&mut self, delta_time: std::time::Duration, _: bool) {
		profiling::scope!("subsystem:physics");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (position, velocity)) in query_bundle.query_mut(&mut world) {
			let velocity_vec = **velocity;
			if velocity_vec.magnitude_squared() > 0.0 {
				*position += velocity_vec * delta_time.as_secs_f32();
			}
		}
	}
}
