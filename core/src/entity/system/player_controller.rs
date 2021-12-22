use crate::{
	account,
	entity::{self, component, ArcLockEntityWorld},
};
use engine::{
	math::nalgebra::{Point3, UnitQuaternion, Vector3},
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

		// TODO: Input action states should be able to be weak-referenced
		// instead of needing to lock the entire input system.
		if let Some(state) =
			engine::input::read().get_user_action(0, crate::input::AXIS_LOOK_HORIZONTAL)
		{
			log::debug!("{}", state.axis_value());
		}

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
			//position.offset = Point3::new(t.cos() * r, 0.0, t.sin() * r) + Vector3::new(0.5, 0.0, 0.5);

			let desired_horizontal_rot = UnitQuaternion::from_axis_angle(
				&-engine::world::global_up(),
				t - 90.0f32.to_radians(),
			);
			//**orientation = desired_horizontal_rot;

			let pi2_radians = 90.0f32.to_radians();
			let additional_horz = UnitQuaternion::from_axis_angle(
				&engine::world::global_forward(),
				delta_time.as_secs_f32() * pi2_radians,
			);
			//**orientation = additional_horz * (**orientation);
		}
	}
}
