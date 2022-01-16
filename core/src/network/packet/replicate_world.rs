use crate::{block, entity::ArcLockEntityWorld, network::storage::ArcLockStorage, world::chunk};
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
use std::sync::{Arc, RwLock};

#[packet_kind(engine::network)]
#[derive(Serialize, Deserialize)]
pub struct ReplicateWorld(WorldUpdate);

#[derive(Serialize, Deserialize)]
enum WorldUpdate {
	Relevancy(ChunkRelevancy),
	Chunk(PartialChunk),
}

/// A partial representation of a [`Chunk`](chunk::Chunk).
/// A chunk is split into multiple structures during replication to avoid significant fragmentation.
///
/// Per laminar/socknet, of a packet exceeds 1450 bytes, it will be fragmented
/// (the maximum transmission unit as of 2016 IPv4 is 1500 bytes).
#[derive(Serialize, Deserialize)]
pub struct PartialChunk {
	/// The coordinate of the chunk this update is for.
	coordinate: Point3<i64>,
	/// A partial list of the blocks in a chunk.
	/// When combined with other partial chunk packets, the full chunk can be reconstructed on the client.
	block_ids: Vec<(Point3<usize>, block::LookupId)>,
}

static PARTIAL_CHUNK_BLOCK_COUNT: usize = 150;
impl PartialChunk {
	pub fn prepared(chunk: &chunk::Chunk) -> Self {
		Self {
			coordinate: *chunk.coordinate(),
			block_ids: Vec::with_capacity(PARTIAL_CHUNK_BLOCK_COUNT),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.block_ids.is_empty()
	}

	pub fn is_full(&self) -> bool {
		self.block_ids.len() >= PARTIAL_CHUNK_BLOCK_COUNT
	}

	pub fn push(&mut self, entry: (Point3<usize>, block::LookupId)) {
		assert!(!self.is_full());
		self.block_ids.push(entry);
	}

	pub fn apply_to(&self, chunk: &mut chunk::Chunk) {
		for &(point, block_id) in self.block_ids.iter() {
			chunk.block_ids.insert(point, block_id);
		}
	}
}

/// In order to avoid fragmentation, one batch of relevancy cannot exceed 60 chunk coordinates.
/// Thats 1450 bytes (MTU wrt 2016 Ipv4) = 24n + 8, where 24 is the size of [i64; 3], 8 is the size of usize/hecs::Entity,
/// and n is the number of coordinates.
static CHUNK_COORDINATES_PER_RELEVANCY: usize = 60;
#[derive(Serialize, Deserialize)]
struct ChunkRelevancy {
	entity: hecs::Entity,
	/// Chunks which used to be, but are no longer, relevant to this client.
	old_chunks: Vec<Point3<i64>>,
}

impl ReplicateWorld {
	pub fn register(
		builder: &mut network::Builder,
		storage: &ArcLockStorage,
		_entity_world: &ArcLockEntityWorld,
	) {
		use mode::Kind::*;

		let client_proc = ReceiveReplicatedWorld {
			storage: storage.clone(),
		};

		builder.register_bundle::<Self>(
			EventProcessors::default()
				.with(Client, client_proc.clone())
				.with(mode::Set::all(), client_proc),
		);
	}

	pub fn fragment_relevancy(entity: hecs::Entity, old_chunks: Vec<Point3<i64>>) -> Vec<Self> {
		let mut packets = Vec::with_capacity(
			(old_chunks.len() / CHUNK_COORDINATES_PER_RELEVANCY)
				+ (old_chunks.len() % CHUNK_COORDINATES_PER_RELEVANCY).min(1),
		);
		let mut group = Vec::with_capacity(CHUNK_COORDINATES_PER_RELEVANCY);
		for coord in old_chunks.into_iter() {
			if group.len() >= CHUNK_COORDINATES_PER_RELEVANCY {
				packets.push(Self(WorldUpdate::Relevancy(ChunkRelevancy {
					entity,
					old_chunks: group,
				})));
				group = Vec::with_capacity(CHUNK_COORDINATES_PER_RELEVANCY);
			}
			group.push(coord);
		}
		if !group.is_empty() {
			packets.push(Self(WorldUpdate::Relevancy(ChunkRelevancy {
				entity,
				old_chunks: group,
			})));
		}
		packets
	}

	fn packets_per_chunk() -> usize {
		let blocks_per_chunk = chunk::DIAMETER.pow(3);
		let partials = blocks_per_chunk / PARTIAL_CHUNK_BLOCK_COUNT;
		let rem = blocks_per_chunk % PARTIAL_CHUNK_BLOCK_COUNT;
		partials + rem.min(1)
	}

	pub fn create_chunk_packets(client_chunk: &chunk::Chunk) -> Vec<Self> {
		profiling::scope!(
			"create_chunk_packets",
			&format!(
				"<{}, {}, {}>",
				client_chunk.coordinate().x,
				client_chunk.coordinate().y,
				client_chunk.coordinate().z
			)
		);
		let mut packets = Vec::with_capacity(Self::packets_per_chunk());
		let mut partial_chunk = PartialChunk::prepared(&client_chunk);
		for (&point, &block_id) in client_chunk.block_ids().iter() {
			if partial_chunk.is_full() {
				packets.push(ReplicateWorld(WorldUpdate::Chunk(partial_chunk)));
				partial_chunk = PartialChunk::prepared(&client_chunk);
			}
			partial_chunk.push((point, block_id));
		}
		if !partial_chunk.is_empty() {
			packets.push(ReplicateWorld(WorldUpdate::Chunk(partial_chunk)));
		}
		packets
	}
}

#[derive(Clone)]
struct ReceiveReplicatedWorld {
	storage: ArcLockStorage,
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
		profiling::scope!("process-packet", "ReplicateWorld");
		match &data.0 {
			WorldUpdate::Relevancy(relevancy) => {
				if let Ok(mut cache) = self.chunk_cache().write() {
					for coord in relevancy.old_chunks.iter() {
						cache.remove(&coord);
					}
				}
			}
			WorldUpdate::Chunk(partial_chunk) => {
				if let Ok(mut cache) = self.chunk_cache().write() {
					match cache.get_loaded(&partial_chunk.coordinate).cloned() {
						Some(arc_chunk) => {
							if let Ok(mut client_chunk) = arc_chunk.write() {
								cache.mark_pending(partial_chunk.coordinate);
								partial_chunk.apply_to(&mut client_chunk);
							}
						}
						None => {
							let mut client_chunk = chunk::Chunk::new(partial_chunk.coordinate);
							partial_chunk.apply_to(&mut client_chunk);
							cache.insert(
								partial_chunk.coordinate,
								Arc::new(RwLock::new(client_chunk)),
							);
						}
					}
				}
			}
		}

		Ok(())
	}
}
