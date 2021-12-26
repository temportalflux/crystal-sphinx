use crate::{
	entity::{
		self,
		component::{self, net, Position},
		ArcLockEntityWorld,
	},
	world::chunk,
};
use engine::EngineSystem;
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
		// TODO: Replicate updates on net::Replicated components
		// - (net::Replicated should have a flag to indicate that it is dirty)
	}
}

impl Replicator {
	#[profiling::function]
	fn replicate_relevant_chunks(&self, arc_world: &ArcLockEntityWorld) {
		use crate::network::packet::{ChunkRelevancy, ReplicateWorld, WorldUpdate};
		use engine::network::{enums::*, packet::Packet, Network};
		type QueryBundle<'c> = hecs::PreparedQuery<(
			&'c component::Position,
			&'c net::Owner,
			&'c mut component::chunk::Relevancy,
		)>;
		// TODO: Replicate relevant chunks to connections based on the position of owner entities
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

			let (new_chunks, old_chunks) = relevancy.get_chunk_diff(&current_chunk);
			// Server side, update the component directly
			relevancy.update_replicated_chunks(current_chunk, &old_chunks, &new_chunks);

			// Get all the coordinates that should be replicated once they are available/loaded.
			// This might be some number of updates after the user moves to a new chunk,
			// so there may be a list of pending chunks for some amount of time.
			let chunks_to_replicate = relevancy.take_pending_chunks();
			let mut updates = Vec::with_capacity(chunks_to_replicate.len());
			if let Ok(chunk_cache) = self.chunk_cache.upgrade().unwrap().read() {
				for coordinate in chunks_to_replicate.iter() {
					// If the chunk is in the cache, then the server has it loaded (to some degree).
					// If not, it needs to go back on the component for the next update cycle.
					match chunk_cache.find(&coordinate) {
						Some(weak_chunk) => {
							if let Ok(server_chunk) = weak_chunk.upgrade().unwrap().read() {
								// Conver the chunk into replication data and add it to the list of things to send.
								let client_chunk = server_chunk.chunk.clone();
								updates.push(ReplicateWorld(WorldUpdate::Chunk(client_chunk)));
								relevancy.mark_as_replicated(*coordinate);
							}
						}
						None => {
							relevancy.mark_as_pending(*coordinate);
						}
					}
				}
			}
			// CotoS skip
			//if *owner.address() == *Network::local_data().address() {
			//	continue;
			//}
			let _ = Network::send_packets(
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
					.with_payloads(&updates[..]),
			);
		}
	}

	#[profiling::function]
	fn replicate_owned_entities(&self, arc_world: &ArcLockEntityWorld) {
		use crate::network::packet::ReplicateEntity;
		use engine::network::{enums::*, packet::Packet, Network};

		let mut world = arc_world.write().unwrap();
		let mut entities_to_replicate = vec![];
		for (id, owner) in world.query_mut::<&mut net::Owner>() {
			if !owner.has_been_replicated() {
				entities_to_replicate.push((id, *owner.address()));
				owner.mark_as_replicated();
			}
		}

		let mut replications = Vec::new();
		let registry = net::Registry::read();
		for (entity, address) in entities_to_replicate.into_iter() {
			let scope_tag = format!("entity:{}", entity.id());
			profiling::scope!("serialize-entity", scope_tag.as_str());

			let entity_ref = world.entity(entity).unwrap();
			match registry.serialize_entity(entity_ref) {
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

	fn _owned_entities(
		&self,
		arc_world: &ArcLockEntityWorld,
	) -> Vec<(hecs::Entity, std::net::SocketAddr, Position)> {
		let world = arc_world.read().unwrap();
		let entities = world
			.query::<(&net::Owner, &Position)>()
			.iter()
			.map(|(entity, (&owner, &position))| (entity, *owner.address(), position))
			.collect::<Vec<_>>();
		entities
	}
}
