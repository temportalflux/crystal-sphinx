use crate::{
	account,
	entity::{self, archetype, component, ArcLockEntityWorld},
};
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
	pub entities: Vec<component::net::SerializedEntity>,
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
		let mut updates = Vec::new();

		for serialized in data.entities.iter() {
			let scope_tag = format!("entity-id:{}", serialized.entity.id());
			profiling::scope!("deserialize-components", scope_tag.as_str());

			let mut builder = hecs::EntityBuilder::default();
			for comp_data in serialized.components.clone().into_iter() {
				let _ = registry.deserialize(comp_data, &mut builder);
			}
			updates.push((serialized.entity, builder));
		}

		if updates.len() > 0 {
			profiling::scope!("spawn-replicated");

			let local_account_id = account::ClientRegistry::read()?
				.active_account()
				.map(|account| account.meta.id.clone());

			let mut world = arc_world.write().unwrap();
			for (entity, mut builder) in updates.into_iter() {
				// If the entity doesn't exist in the world, spawn it with the components.
				// Otherwise, replace any existing components with the same types with the new data.
				// Example: Dedicated or Integrated Server spawns an entity and a Client receives
				//          the update for the first time. Client doesn't have the entity in its
				//          world yet, so it and its components are spawned.
				// Integrated Client-Server might spawn an entity, but it should never send the packet to itself.
				let bundle = builder.build();
				if !world.contains(entity) {
					world.spawn_at(entity, bundle);
				} else {
					let _ = world.insert(entity, bundle);
				}

				match (local_account_id, builder.get::<component::User>()) {
					(Some(local_id), Some(user)) => {
						// If the account ids match, then this entity is the local player's avatar
						if *user.id() == local_id {
							let entity_ref = world.entity(entity).unwrap();
							if let Some(mut builder) =
								archetype::player::Client::from(Some(entity_ref)).build()
							{
								let _ = world.insert(entity, builder.build());
							}
						}
					}
					_ => {}
				}
			}
		}

		Ok(())
	}
}
