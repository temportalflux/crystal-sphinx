use crate::{
	account,
	entity::{self, component, ArcLockEntityWorld},
};
use engine::{
	input,
	math::nalgebra::{Point3, Unit, UnitQuaternion, Vector3},
	world, EngineSystem,
};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::User,
	&'c mut component::Position,
	&'c mut component::Orientation,
)>;

enum RotationOrder {
	First, Second,
}

struct LookAction {
	action: input::action::WeakLockState,
	side: RotationOrder,
	axis: Unit<Vector3<f32>>,
}

impl LookAction {
	fn take_value(&self) -> f32 {
		if let Some(arc_state) = self.action.upgrade() {
			arc_state.write().unwrap().take_value() as f32
		} else {
			0.0
		}
	}
	
	fn concat_into(&self, value: f32, orientation: &mut UnitQuaternion<f32>) {
		if value.abs() > std::f32::EPSILON {
			let rot = UnitQuaternion::from_axis_angle(
				&self.axis,
				value * 90.0f32.to_radians(),
			);
			match self.side {
				RotationOrder::First => {
					*orientation = (*orientation) * rot;
				}
				RotationOrder::Second => {
					*orientation = rot * (*orientation);
				}
			}
		}
	}
}

pub struct PlayerController {
	world: Weak<RwLock<entity::World>>,
	account_id: account::Id,
	look_actions: Vec<LookAction>,
	time: f32,
}

impl PlayerController {
	pub fn new(
		world: &ArcLockEntityWorld,
		account_id: account::Id,
		arc_user: &input::ArcLockUser,
	) -> Self {
		let get_action = |id| input::User::get_action_in(&arc_user, id).unwrap();

		Self {
			world: Arc::downgrade(&world),
			account_id,
			look_actions: vec![
				LookAction {
					action: get_action(crate::input::AXIS_LOOK_VERTICAL),
					side: RotationOrder::First,
					axis: -world::global_right(),
				},
				LookAction {
					action: get_action(crate::input::AXIS_LOOK_HORIZONTAL),
					side: RotationOrder::Second,
					axis: world::global_up(),
				},
			],
			time: 0.0,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for PlayerController {
	fn update(&mut self, delta_time: std::time::Duration, _has_focus: bool) {
		profiling::scope!("subsystem:player_controller");

		let input_values = self.look_actions.iter().map(|action| action.take_value()).collect::<Vec<_>>();

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

			/* Rotate around <0.5, 0, 0.5>
			let r = 3.0;
			self.time += delta_time.as_secs_f32();
			let t = self.time / 10.0;
			let t = t * std::f32::consts::PI * 2.0;
			position.offset = Point3::new(t.cos() * r, 0.0, t.sin() * r) + Vector3::new(0.5, 0.0, 0.5);

			let desired_horizontal_rot = UnitQuaternion::from_axis_angle(
				&-engine::world::global_up(),
				t - 90.0f32.to_radians(),
			);
			**orientation = desired_horizontal_rot;
			*/
			
			for (look_action, value) in self.look_actions.iter().zip(input_values.iter()) {
				look_action.concat_into(*value, &mut (**orientation));
			}
		}
	}
}
