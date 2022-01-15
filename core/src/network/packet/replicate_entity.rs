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
pub struct Packet {
	pub operations: Vec<Operation>,
}

#[derive(Serialize, Deserialize)]
pub enum Operation {
	Replicate(component::binary::SerializedEntity),
	Destroy(hecs::Entity),
}

impl Packet {
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

impl PacketProcessor<Packet> for ReceiveReplicatedEntity {
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut Packet,
		_connection: &Connection,
		_guarantee: &packet::Guarantee,
		_local_data: &network::LocalData,
	) -> VoidResult {
		profiling::scope!("process-packet", "ReplicateEntity");

		let arc_world = match self.entity_world.upgrade() {
			Some(arc) => arc,
			None => return Ok(()),
		};

		let registry = component::Registry::read();
		let mut entities_to_spawn = Vec::new();
		let mut entities_to_despawn = Vec::new();

		for operation in data.operations.iter() {
			match operation {
				Operation::Replicate(serialized) => {
					profiling::scope!(
						"deserialize-components",
						&format!("entity-id:{}", serialized.entity.id())
					);

					let mut builder = hecs::EntityBuilder::default();
					for comp_data in serialized.components.clone().into_iter() {
						let type_id = registry.get_type_id(&comp_data.id).unwrap();
						if let Some(registered) = registry.find(&type_id) {
							match registered.get::<component::binary::Registration>() {
								Some(binary_registration) => {
									let _ = binary_registration
										.deserialize(comp_data.data, &mut builder);
								}
								None => {
									log::warn!(target: "ReplicateEntity", "Failed to deserialize, no binary registration found for component({})", comp_data.id);
								}
							}
						}
					}
					entities_to_spawn.push((serialized.entity, builder));
				}
				Operation::Destroy(entity) => {
					entities_to_despawn.push(*entity);
				}
			}
		}

		if !entities_to_despawn.is_empty() {
			profiling::scope!("despawn-replicated");
			let mut world = arc_world.write().unwrap();
			for entity in entities_to_despawn.into_iter() {
				log::debug!(target: "ReplicateEntity", "Despawning replicated entity {}", entity.id());
				let _ = world.despawn(entity);
			}
		}

		if entities_to_spawn.len() > 0 {
			profiling::scope!("spawn-replicated");

			let local_account_id = account::ClientRegistry::read()?
				.active_account()
				.map(|account| account.meta.id.clone());

			let mut world = arc_world.write().unwrap();
			for (entity, mut builder) in entities_to_spawn.into_iter() {
				log::debug!(target: "ReplicateEntity", "Spawning replicated entity {}", entity.id());

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

				match (local_account_id, builder.get::<component::OwnedByAccount>()) {
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
