use crate::{
	entity::{
		self,
		component::{self, binary, Position},
		ArcLockEntityWorld,
	},
	world::chunk,
};
use engine::{utility::AnyError, EngineSystem};
use std::sync::{Arc, RwLock, Weak};

/// Replicates entities on the Server to connected Clients while they are net-relevant.
pub struct Replicator {
	world: Weak<RwLock<entity::World>>,
	chunk_cache: chunk::WeakLockServerCache,
}

impl Replicator {
	pub fn new(chunk_cache: chunk::WeakLockServerCache, world: &ArcLockEntityWorld) -> Self {
		Self {
			chunk_cache,
			world: Arc::downgrade(&world),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for Replicator {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!("subsystem:replicator");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};

		// Sends entities to connections which own/control them
		self.replicate_owned_entities(&arc_world);

		// Replicate relevant chunks to connections based on the position of owner entities
		self.replicate_relevant_chunks(&arc_world);

		// TODO: Replicate relevant entities to other connections
		// TODO: Destroy entities from other connecctions when they are removed from the world
		// TODO: Replicate updates on net::BinarySerializable components
		// - (net::BinarySerializable should have a flag to indicate that it is dirty)
	}
}

impl Replicator {
	#[profiling::function]
	fn replicate_relevant_chunks(&self, arc_world: &ArcLockEntityWorld) {
		use crate::network::packet::{ChunkRelevancy, ReplicateWorld, WorldUpdate};
		use engine::network::{enums::*, packet::Packet, Network};
		type QueryBundle<'c> = hecs::PreparedQuery<(
			&'c component::Position,
			&'c component::OwnedByConnection,
			&'c mut component::chunk::Relevancy,
		)>;

		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (entity, (position, owner, relevancy)) in query_bundle.query_mut(&mut world) {
			// Chunk coordinate of where the entity is now
			let current_chunk = *position.chunk();
			// Chunk coordinate of the last replication
			let previous_chunk = *relevancy.chunk();
			// If the current chunk is unchanged and there are no
			// chunks to replicate, then this can early out.
			if current_chunk == previous_chunk && relevancy.has_replicated_all() {
				continue;
			}

			profiling::scope!("replicate-chunks", &format!("entity={} address={}", entity.id(), owner.address()));

			let (new_chunks, old_chunks) = relevancy.get_chunk_diff(&current_chunk);
			// Server side, update the component directly
			relevancy.update_replicated_chunks(current_chunk, &old_chunks, &new_chunks);

			let old_chunks = {
				profiling::scope!("sort-old-chunks");
				let mut old_chunks = old_chunks.into_iter().collect::<Vec<_>>();
				old_chunks.sort_by(|a, b| {
					let a_dist = (a - current_chunk).cast::<f32>().magnitude_squared();
					let b_dist = (b - current_chunk).cast::<f32>().magnitude_squared();
					b_dist.partial_cmp(&a_dist).unwrap()
				});
				old_chunks
			};

			log::debug!(
				"update chunks: {} new, {} old",
				new_chunks.len(),
				old_chunks.len()
			);

			// Get all the coordinates that should be replicated once they are available/loaded.
			// This might be some number of updates after the user moves to a new chunk,
			// so there may be a list of pending chunks for some amount of time.
			let chunks_to_replicate = relevancy.take_pending_chunks();
			let chunks_to_replicate = {
				profiling::scope!("sort-pending-chunks");
				let mut chunks_to_replicate = chunks_to_replicate.into_iter().collect::<Vec<_>>();
				chunks_to_replicate.sort_by(|a, b| {
					let a_dist = (a - current_chunk).cast::<f32>().magnitude_squared();
					let b_dist = (b - current_chunk).cast::<f32>().magnitude_squared();
					a_dist.partial_cmp(&b_dist).unwrap()
				});
				chunks_to_replicate
			};
			let mut updates = Vec::with_capacity(chunks_to_replicate.len());
			if let Ok(chunk_cache) = self.chunk_cache.upgrade().unwrap().read() {
				for coordinate in chunks_to_replicate.into_iter() {
					profiling::scope!("create-chunk-packet", &format!("<{}, {}, {}>", coordinate.x, coordinate.y, coordinate.z));
					// If the chunk is in the cache, then the server has it loaded (to some degree).
					// If not, it needs to go back on the component for the next update cycle.
					match chunk_cache.find(&coordinate) {
						Some(weak_chunk) => {
							let arc_chunk = weak_chunk.upgrade().unwrap();
							let server_chunk = arc_chunk.read().unwrap();
							// Conver the chunk into replication data and add it to the list of things to send.
							let client_chunk = server_chunk.chunk.clone();
							updates.push(ReplicateWorld(WorldUpdate::Chunk(client_chunk)));
							relevancy.mark_as_replicated(coordinate);
						}
						None => {
							relevancy.mark_as_pending(coordinate);
						}
					}
				}
			}

			log::debug!("Replicating {} chunk updates", updates.len());

			// CotoS skip
			//if *owner.address() == *Network::local_data().address() {
			//	continue;
			//}
			let packets = {
				profiling::scope!("build-packets");
				Packet::builder()
					.with_address(*owner.address())
					.unwrap()
					// TODO: Should the Integrated Client-Server send to itself?
					//.ignore_local_address()
					.with_guarantee(Reliable + Ordered)
					// Notify client what chunks are no longer relevant (can be dropped),
					// and what chunks will be incoming over the network shortly.
					.with_payload(&ReplicateWorld(WorldUpdate::Relevancy(ChunkRelevancy {
						entity,
						new_chunks,
						old_chunks,
						origin: current_chunk,
					})))
					// Send each chunk update in its own Reliably-Ordered packet,
					// which is garunteed to be received by clients after the initial update.
					.with_payloads(&updates[..])
			};
			let _ = Network::send_packets(
				packets
			);
		}
	}

	#[profiling::function]
	fn replicate_owned_entities(&self, arc_world: &ArcLockEntityWorld) {
		use crate::network::packet::ReplicateEntity;
		use engine::network::{enums::*, packet::Packet, Network};

		let mut world = arc_world.write().unwrap();
		let mut entities_to_replicate = vec![];
		for (id, owner) in world.query_mut::<&mut component::OwnedByConnection>() {
			if !owner.has_been_replicated() {
				entities_to_replicate.push((id, *owner.address()));
				owner.mark_as_replicated();
			}
		}

		let mut replications = Vec::new();
		let registry = component::Registry::read();
		for (entity, address) in entities_to_replicate.into_iter() {
			let scope_tag = format!("entity:{}", entity.id());
			profiling::scope!("serialize-entity", scope_tag.as_str());

			let entity_ref = world.entity(entity).unwrap();
			match self.serialize_entity(&registry, entity_ref) {
				Ok(serialized) => {
					replications.push((address.clone(), serialized));
				}
				Err(err) => {
					log::error!(target: "entity-replicator", "Encountered error while serializing entity: {}", err)
				}
			}
		}

		for (address, serialized) in replications.into_iter() {
			let _ = Network::send_packets(
				Packet::builder()
					.with_address(address)
					.unwrap()
					// Integrated Client-Server should not sent to itself
					.ignore_local_address()
					.with_guarantee(Reliable + Ordered)
					.with_payload(&ReplicateEntity {
						entities: vec![serialized],
					}),
			);
		}
	}

	fn serialize_entity(
		&self,
		registry: &component::Registry,
		entity_ref: hecs::EntityRef<'_>,
	) -> Result<binary::SerializedEntity, AnyError> {
		let mut serialized_components = Vec::new();
		for type_id in entity_ref.component_types() {
			// TODO: Implement a Replicated trait for the components which should actually be replicated (instead of using binary::Serializable as the marker).
			if let Some(registered) = registry.find(&type_id) {
				if let Some(binary_registration) = registered.get::<binary::Registration>() {
					match binary_registration.serialize(&entity_ref)? {
						Some(serialized) => {
							serialized_components.push(serialized);
						}
						None => {} // The component didn't actually exist on the entity
					}
				}
			}
		}
		Ok(binary::SerializedEntity {
			entity: entity_ref.entity(),
			components: serialized_components,
		})
	}

	fn _owned_entities(
		&self,
		arc_world: &ArcLockEntityWorld,
	) -> Vec<(hecs::Entity, std::net::SocketAddr, Position)> {
		let world = arc_world.read().unwrap();
		let entities = world
			.query::<(&component::OwnedByConnection, &Position)>()
			.iter()
			.map(|(entity, (&owner, &position))| (entity, *owner.address(), position))
			.collect::<Vec<_>>();
		entities
	}
}
