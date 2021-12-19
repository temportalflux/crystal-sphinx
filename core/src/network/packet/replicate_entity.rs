use crate::entity::{self, component, ArcLockEntityWorld};
use engine::{
	network::{
		self,
		connection::Connection,
		event, mode, packet, packet_kind,
		processor::{EventProcessors, PacketProcessor, Processor},
	},
	utility::VoidResult,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock, Weak};

#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct ReplicateEntity {
	pub entity: hecs::Entity,
	pub serialized_components: Vec<component::net::SerializedComponent>,
}

impl ReplicateEntity {
	pub fn register(builder: &mut network::Builder, entity_world: &ArcLockEntityWorld) {
		use mode::Kind::*;

		let client_proc = ReceiveReplicatedEntity {
			entity_world: Arc::downgrade(&entity_world),
		};

		builder.register_bundle::<Self>(
			EventProcessors::default()
				.with(Client, client_proc.clone())
				.with(mode::Set::all(), client_proc),
		);
	}
}

#[derive(Clone)]
struct ReceiveReplicatedEntity {
	entity_world: Weak<RwLock<entity::World>>,
}

impl Processor for ReceiveReplicatedEntity {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &network::LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<ReplicateEntity> for ReceiveReplicatedEntity {
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut ReplicateEntity,
		_connection: &Connection,
		_guarantee: &packet::Guarantee,
		_local_data: &network::LocalData,
	) -> VoidResult {
		profiling::scope!("process-packet", "ReplicateEntity");

		let arc_world = match self.entity_world.upgrade() {
			Some(arc) => arc,
			None => return Ok(()),
		};
		
		let registry = component::net::Registry::read();
		let mut builder = hecs::EntityBuilder::default();
		for comp_data in data.serialized_components.clone().into_iter() {
			let _ = registry.deserialize(comp_data, &mut builder);
		}

		if let Ok(mut world) = arc_world.write() {
			// Dedicated Clients wont run this logic,
			// but because client-on-top-of-server is supported,
			// its possible for the entity to already be in the shared-world.
			if !world.contains(data.entity) {
				world.spawn_at(data.entity, builder.build());
			}
		}

		// TODO: Attach owner-client-only components if the replicated entity is the client-player

		Ok(())
	}
}
