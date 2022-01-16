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
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, RwLock, Weak},
};

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
			server_to_client_id: Arc::new(Mutex::new(HashMap::new())),
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
	server_to_client_id: Arc<Mutex<HashMap<hecs::Entity, hecs::Entity>>>,
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
							match registered.get_ext::<component::binary::Registration>() {
								Some(binary_registration) => {
									if let Err(err) = binary_registration
										.deserialize(comp_data.data, &mut builder)
									{
										log::error!(target: "ReplicateEntity", "Encountered error while deserializing component {}, {}", comp_data.id, err);
									}
								}
								None => {
									log::warn!(target: "ReplicateEntity", "Failed to deserialize, no binary registration found for component({})", comp_data.id);
								}
							}
						} else {
							log::error!(target: "ReplicateEntity", "Failed to find registration for serialized component {}", comp_data.id);
						}
					}
					entities_to_spawn.push((serialized.entity, builder));
				}
				Operation::Destroy(server_entity) => {
					entities_to_despawn.push(*server_entity);
				}
			}
		}

		let mut entity_map = self.server_to_client_id.lock().unwrap();

		if !entities_to_despawn.is_empty() {
			profiling::scope!("despawn-replicated");
			let mut world = arc_world.write().unwrap();
			for server_entity in entities_to_despawn.into_iter() {
				if let Some(client_entity) = entity_map.remove(&server_entity) {
					let _ = world.despawn(client_entity);
				}
			}
		}

		if entities_to_spawn.len() > 0 {
			profiling::scope!("spawn-replicated");

			let local_account_id = account::ClientRegistry::read()?
				.active_account()
				.map(|account| account.meta.id.clone());

			let mut world = arc_world.write().unwrap();
			let registry = component::Registry::read();
			for (server_entity, mut builder) in entities_to_spawn.into_iter() {
				let is_locally_owned =
					match (local_account_id, builder.get::<component::OwnedByAccount>()) {
						// If the account ids match, then this entity is the local player's avatar
						(Some(local_id), Some(user)) => *user.id() == local_id,
						_ => false,
					};

				// If the entity doesn't exist in the world, spawn it with the components.
				// Otherwise, replace any existing components with the same types with the new data.
				// Example: Dedicated or Integrated Server spawns an entity and a Client receives
				//          the update for the first time. Client doesn't have the entity in its
				//          world yet, so it and its components are spawned.
				// Integrated Client-Server might spawn an entity, but it should never send the packet to itself.
				match entity_map.get(&server_entity) {
					None => {
						if is_locally_owned {
							builder = archetype::player::Client::apply_to(builder);
						}
						let client_entity = world.spawn(builder.build());
						entity_map.insert(server_entity, client_entity);
					}
					Some(client_entity) => {
						let mut missing_components = {
							let entity_ref = world.entity(*client_entity)?;
							let mut missing_components = hecs::EntityBuilder::new();
							for type_id in builder.component_types() {
								let registered = match registry.find(&type_id) {
									Some(reg) => reg,
									None => {
										log::error!(target: "ReplicateEntity", "Failed to find registration for entity component {:?}", type_id);
										continue;
									}
								};
								let network_ext = match registered
									.get_ext::<component::network::Registration>()
								{
									Some(ext) => ext,
									None => {
										log::error!(target: "ReplicateEntity", "Entity component {} was replicated but does not have the network replication registration extension.", registered.display_name());
										continue;
									}
								};
								// Read the data from the replicated component into the existing entity.
								if !registered.is_in_entity(&entity_ref) {
									// cache the missing component to the builder for adding all missing components at once
									network_ext
										.clone_into_builder(&builder, &mut missing_components);
								} else {
									// read the data from the replicated component into the existing component
									network_ext.on_replication(
										&builder,
										&entity_ref,
										is_locally_owned,
									);
								}
							}
							missing_components
						};
						if missing_components.component_types().count() > 0 {
							let _ = world.insert(*client_entity, missing_components.build());
						}
					}
				}
			}
		}

		Ok(())
	}
}
