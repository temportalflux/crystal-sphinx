use crate::{
	account,
	entity::{self, component, ArcLockEntityWorld},
};
use engine::{
	math::nalgebra::{Point3, Vector3, UnitQuaternion},
	EngineSystem,
};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::User,
	&'c mut component::Position,
	&'c mut component::Orientation,
)>;

pub struct PlayerController {
	world: Weak<RwLock<entity::World>>,
	account_id: account::Id,
	time: f32,
}

impl PlayerController {
	pub fn new(world: &ArcLockEntityWorld, account_id: account::Id) -> Self {
		Self {
			world: Arc::downgrade(&world),
			account_id,
			time: 0.0,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for PlayerController {
	fn update(&mut self, delta_time: std::time::Duration) {
		profiling::scope!("subsystem:player_controller");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (entity_user, position, orientation)) in query_bundle.query_mut(&mut world) {
			// Only control the entity which is owned by the local player
			if *entity_user.id() != self.account_id {
				continue;
			}

			let r = 3.0;
			self.time += delta_time.as_secs_f32();
			let t = self.time / 10.0;
			let t = t * std::f32::consts::PI * 2.0;
			position.offset = Point3::new(t.cos() * r, 0.0, t.sin() * r) + Vector3::new(0.5, 0.0, 0.5);
			**orientation = UnitQuaternion::from_axis_angle(
				&-engine::world::global_up(),
				t - 90.0f32.to_radians()
			);
		}
	}
}
