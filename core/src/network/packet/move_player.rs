use crate::entity::{self, ArcLockEntityWorld};
use engine::{
	math::nalgebra::{UnitQuaternion, Vector3},
	network::{
		self,
		connection::Connection,
		event, mode, packet, packet_kind,
		processor::{EventProcessors, PacketProcessor, Processor},
	},
	utility::Result,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock, Weak};

static LOG: &'static str = "MovePlayer";

/// Packet sent from Dedicated Client to Server telling the server
/// what the velocity and orientation of the player is (based on user input).
#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct MovePlayer {
	pub server_entity: hecs::Entity,
	pub velocity: Vector3<f32>,
	pub orientation: UnitQuaternion<f32>,
}

impl MovePlayer {
	pub fn register(builder: &mut network::Builder, entity_world: &ArcLockEntityWorld) {
		use mode::Kind::*;

		let server_proc = RequestReceived {
			entity_world: Arc::downgrade(&entity_world),
		};

		builder.register_bundle::<Self>(
			EventProcessors::default()
				.with(Server, server_proc.clone())
				.with(mode::Set::all(), server_proc),
		);
	}
}

#[derive(Clone)]
struct RequestReceived {
	entity_world: Weak<RwLock<entity::World>>,
}

impl Processor for RequestReceived {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &network::LocalData,
	) -> Result<()> {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<MovePlayer> for RequestReceived {
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut MovePlayer,
		_connection: &Connection,
		_guarantee: &packet::Guarantee,
		_local_data: &network::LocalData,
	) -> Result<()> {
		use entity::component::{physics::linear, Orientation};
		profiling::scope!("process-packet", LOG);

		let arc_world = match self.entity_world.upgrade() {
			Some(arc) => arc,
			None => return Ok(()),
		};

		let world = arc_world.read().unwrap();
		if let Ok(entity_ref) = world.entity(data.server_entity) {
			if let Some(mut velocity) = entity_ref.get_mut::<linear::Velocity>() {
				**velocity = data.velocity;
			}
			if let Some(mut orientation) = entity_ref.get_mut::<Orientation>() {
				**orientation = data.orientation;
			}
		}

		Ok(())
	}
}
