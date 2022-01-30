use crate::{
	account,
	common::network::mode,
	entity::{self, component, ArcLockEntityWorld},
	network::packet::MovePlayer,
};
use engine::{
	input,
	math::nalgebra::{Unit, UnitQuaternion, Vector3},
	network, world, EngineSystem,
};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::OwnedByAccount,
	&'c mut component::physics::linear::Velocity,
	&'c mut component::Orientation,
	&'c mut component::network::Replicated,
)>;

enum RotationOrder {
	First,
	Second,
}

struct MoveAction {
	action: input::action::WeakLockState,
	direction: Unit<Vector3<f32>>,
	is_global: bool,
}

impl MoveAction {
	fn value(&self) -> f32 {
		if let Some(arc_state) = self.action.upgrade() {
			arc_state.read().unwrap().value() as f32
		} else {
			0.0
		}
	}
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
			let rot = UnitQuaternion::from_axis_angle(&self.axis, value * 90.0f32.to_radians());
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
	move_speed: f32,
	move_actions: Vec<MoveAction>,
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
			move_speed: 4.0,
			move_actions: vec![
				MoveAction {
					action: get_action(crate::input::AXIS_MOVE),
					direction: world::global_forward(),
					is_global: false,
				},
				MoveAction {
					action: get_action(crate::input::AXIS_STRAFE),
					direction: world::global_right(),
					is_global: false,
				},
				MoveAction {
					action: get_action(crate::input::AXIS_FLY),
					direction: world::global_up(),
					is_global: true,
				},
			],
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for PlayerController {
	fn update(&mut self, _delta_time: std::time::Duration, has_focus: bool) {
		if !has_focus {
			return;
		}

		profiling::scope!("subsystem:player_controller");

		let look_values = self
			.look_actions
			.iter()
			.map(|action| action.take_value())
			.collect::<Vec<_>>();
		let move_values = self
			.move_actions
			.iter()
			.map(|action| action.value())
			.collect::<Vec<_>>();

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (entity_user, velocity, orientation, replicated)) in
			query_bundle.query_mut(&mut world)
		{
			// Only control the entity which is owned by the local player
			if *entity_user.id() != self.account_id {
				continue;
			}

			let prev_velocity = **velocity;
			let prev_orientation = **orientation;

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

			// Its OK to modify the velocity and orientation of the player on a Dedicated Client.
			// A couple reasons why:
			// 1. Clients need to have local prediction while their movement request is in-flight,
			//    and thus need to update the physics so it gets simulated locally.
			// 2. The relevant components will be authoritatively replicated from the server,
			//    so there is no risk of client-authority here.

			**velocity = Vector3::new(0.0, 0.0, 0.0);
			for (move_action, &value) in self.move_actions.iter().zip(move_values.iter()) {
				if value.abs() > std::f32::EPSILON {
					let mut direction = *move_action.direction;
					if !move_action.is_global {
						direction = (**orientation) * direction;
						direction.y = 0.0;
					}
					direction = direction.normalize();
					**velocity += direction * value * self.move_speed;
				}
			}

			for (look_action, value) in self.look_actions.iter().zip(look_values.iter()) {
				look_action.concat_into(*value, &mut (**orientation));
			}

			if mode::get() == mode::Kind::Client {
				const SIG_VEL_MAGNITUDE: f32 = 0.05;
				const SIG_ORIENTATION_ANGLE_DIFF: f32 = 0.005;

				let mut has_significantly_changed = false;
				if (**velocity - prev_velocity).magnitude_squared() >= SIG_VEL_MAGNITUDE.powi(2) {
					has_significantly_changed = true;
				}
				if prev_orientation.angle_to(&**orientation) >= SIG_ORIENTATION_ANGLE_DIFF {
					has_significantly_changed = true;
				}

				if has_significantly_changed {
					/* TODO: Reimplement with new networking
					use network::{enums::*, packet::Packet, Network};
					assert!(replicated.get_id_on_server().is_some());
					let server_entity = *replicated.get_id_on_server().unwrap();
					let _ = Network::send_packets(
						Packet::builder()
							.with_mode(ToServer)
							.with_guarantee(Unreliable + Sequenced)
							.with_payload(&MovePlayer {
								server_entity,
								velocity: **velocity,
								orientation: **orientation,
							}),
					);
					*/
				}
			}
		}
	}
}
