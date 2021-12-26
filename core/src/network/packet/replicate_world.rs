use crate::{
	entity::{self, ArcLockEntityWorld},
	network::storage::ArcLockStorage,
	world::chunk,
};
use engine::{
	math::nalgebra::Point3,
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
	collections::HashSet,
	sync::{Arc, RwLock, Weak},
};

#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct ReplicateWorld(pub WorldUpdate);

#[derive(Serialize, Deserialize)]
pub enum WorldUpdate {
	Relevancy(ChunkRelevancy),
	Chunk(chunk::Chunk),
}

#[derive(Serialize, Deserialize)]
pub struct ChunkRelevancy {
	pub entity: hecs::Entity,
	/// Chunks which used to be, but are no longer, relevant to this client.
	pub old_chunks: HashSet<Point3<i64>>,
	/// The new "center" of the relevant-chunks list.
	pub origin: Point3<i64>,
	/// Chunks which are now relevant to this client.
	/// Data for these chunks will arrive shortly.
	pub new_chunks: HashSet<Point3<i64>>,
}

impl ReplicateWorld {
	pub fn register(
		builder: &mut network::Builder,
		storage: &ArcLockStorage,
		entity_world: &ArcLockEntityWorld,
	) {
		use mode::Kind::*;

		let client_proc = ReceiveReplicatedWorld {
			storage: storage.clone(),
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
struct ReceiveReplicatedWorld {
	storage: ArcLockStorage,
	entity_world: Weak<RwLock<entity::World>>,
}

impl ReceiveReplicatedWorld {
	fn chunk_cache(&self) -> chunk::ArcLockClientCache {
		let storage = self.storage.read().unwrap();
		let arc_client = storage.client().as_ref().unwrap();
		let client = arc_client.read().unwrap();
		client.chunk_cache().clone()
	}
}

impl Processor for ReceiveReplicatedWorld {
	fn process(
		&self,
		kind: &event::Kind,
		data: &mut Option<event::Data>,
		local_data: &network::LocalData,
	) -> VoidResult {
		self.process_as(kind, data, local_data)
	}
}

impl PacketProcessor<ReplicateWorld> for ReceiveReplicatedWorld {
	fn process_packet(
		&self,
		_kind: &event::Kind,
		data: &mut ReplicateWorld,
		_connection: &Connection,
		_guarantee: &packet::Guarantee,
		_local_data: &network::LocalData,
	) -> VoidResult {
		use entity::component::chunk::Relevancy;
		profiling::scope!("process-packet", "ReplicateWorld");
		match &data.0 {
			WorldUpdate::Relevancy(relevancy) => {
				let arc_world = self.entity_world.upgrade().unwrap();
				if let Ok(mut world) = arc_world.write() {
					match world.query_one_mut::<&mut Relevancy>(relevancy.entity) {
						Ok(e_relevancy) => {
							e_relevancy.update_with_replicated(
								relevancy.origin,
								&relevancy.old_chunks,
								&relevancy.new_chunks,
							);
						}
						Err(_) => {
							log::warn!(target: "replicator", "Client has not yet received its owner entity, but already has receved a world update.");
						}
					}
				}
				if let Ok(mut cache) = self.chunk_cache().write() {
					for coord in relevancy.old_chunks.iter() {
						cache.remove(&coord);
					}
					for &coord in relevancy.new_chunks.iter() {
						cache.insert_pending(coord);
					}
				}
			}
			WorldUpdate::Chunk(client_chunk) => {
				if let Ok(mut cache) = self.chunk_cache().write() {
					cache.insert(
						&client_chunk.coordinate(),
						Arc::new(RwLock::new(client_chunk.clone())),
					);
				}
			}
		}

		Ok(())
	}
}
