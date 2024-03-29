use super::{relevancy, EntityOperation};
use crate::{
	client::world::chunk::OperationSender as ClientChunkOperationSender,
	common::network::replication::{self, entity},
	entity::{component::binary, system::replicator::ChunksByRelevance},
};
use socknet::connection::Connection;
use std::{collections::HashMap, net::SocketAddr, sync::Weak};

/// Stateful information about what is relevant to a specific client.
///
/// Also servers as the connective tissue between the
/// [`replicator`](super::Replicator) system and the async task,
/// which dispatches entity replication data to a given client.
///
/// Its lifetime is owned by the replicator system.
pub struct Handle {
	channel: UpdateChannel,
	chunk_relevance: relevancy::Relevance,
	entity_relevance: relevancy::Relevance,
	relevancy_log: String,
	pending_chunks: ChunksByRelevance,
}

enum UpdateChannel {
	Remote(relevancy::WorldUpdateSender, entity::SendUpdate),
	Local(ClientChunkOperationSender),
}

impl Handle {
	pub fn new_local(
		address: &SocketAddr,
		chunk_sender: ClientChunkOperationSender,
	) -> anyhow::Result<Self> {
		// We do not create a replication stream for "local" connections,
		// where the defn of local in this context is the same application,
		// aka an Integrated Server / Client-on-top-of-Server situation.
		// Since a CotoS has a shared world between client and server,
		// there is no point in wasting cycles pretending to replicate data.
		Ok(Self::new(address, UpdateChannel::Local(chunk_sender)))
	}

	pub fn new_remote(address: &SocketAddr, connection: &Weak<Connection>) -> anyhow::Result<Self> {
		let (send_world_rel, recv_world_rel) = engine::channels::future::unbounded();
		let (send_entities, recv_entities) = engine::channels::future::unbounded();
		let (send_chunks, recv_chunks) = engine::channels::future::unbounded();

		replication::entity::spawn(connection.clone(), recv_entities)?;
		replication::world::relevancy::spawn(connection.clone(), recv_world_rel, send_chunks)?;
		for i in 0..10 {
			replication::world::chunk::spawn(connection.clone(), i, recv_chunks.clone())?;
		}

		let channel = UpdateChannel::Remote(send_world_rel, send_entities);

		Ok(Self::new(address, channel))
	}

	fn new(address: &SocketAddr, channel: UpdateChannel) -> Self {
		let relevancy_log = format!("relevancy[{}]", address);
		Self {
			channel,
			chunk_relevance: relevancy::Relevance::default(),
			entity_relevance: relevancy::Relevance::default(),
			relevancy_log,
			pending_chunks: ChunksByRelevance::new(),
		}
	}

	pub fn send_relevance_updates(&mut self, updates: Vec<relevancy::Update>) {
		profiling::scope!(
			"send_relevance_updates",
			&format!("count: {}", updates.len())
		);
		for update in updates.into_iter() {
			match update {
				relevancy::Update::World(update) => {
					let mut relevance_change = None;
					if let relevancy::WorldUpdate::Relevance(relevance) = &update {
						if *relevance == self.chunk_relevance {
							continue;
						}
						relevance_change = Some(relevance.clone());
					}
					self.send_world_update(update);
					if let Some(relevance) = relevance_change {
						self.chunk_relevance = relevance;
					}
				}
				relevancy::Update::Entity(relevance) => {
					self.entity_relevance = relevance;
				}
			}
		}
	}

	fn send_world_update(&mut self, update: relevancy::WorldUpdate) {
		use engine::channels::future::TrySendError;
		match &self.channel {
			UpdateChannel::Remote(send_world_rel, _) => {
				if let Err(err) = send_world_rel.try_send(update) {
					match err {
						TrySendError::Full(_) => {
							log::error!(target: &self.relevancy_log, "Failed to send relevancy delta, unbounded async channel is full. This should never happen.");
						}
						TrySendError::Closed(_) => {
							log::error!(target: &self.relevancy_log, "Failed to send relevancy delta, channel is closed. This should never happen because the channel can only be closed if the stream handle is dropped.");
						}
					}
				}
			}
			UpdateChannel::Local(chunk_sender) => {
				use crate::client::world::chunk::Operation;
				match update {
					relevancy::WorldUpdate::Relevance(relevance) => {
						let mut chunks_to_remove = ChunksByRelevance::new();
						chunks_to_remove.insert_cuboids(
							self.chunk_relevance.difference(&relevance),
							&self.chunk_relevance,
						);
						for coord in chunks_to_remove.into_sorted().into_iter() {
							let _ = chunk_sender.try_send(Operation::Remove(coord));
						}
					}
					relevancy::WorldUpdate::Chunks(new_chunks) => {
						for weak_chunk in new_chunks.into_iter() {
							let operation = match weak_chunk.upgrade() {
								Some(arc_chunk) => {
									let server_chunk = arc_chunk.read().unwrap();
									let coord = server_chunk.chunk.coordinate.clone();
									let updates = server_chunk
										.chunk
										.block_ids
										.iter()
										.map(|(offset, id)| (*offset, *id))
										.collect::<Vec<_>>();
									Operation::Insert(coord, updates)
								}
								None => continue,
							};
							let _ = chunk_sender.try_send(operation);
						}
					}
				}
			}
		}
	}

	pub fn pending_chunks(&self) -> &ChunksByRelevance {
		&self.pending_chunks
	}

	pub fn pending_chunks_mut(&mut self) -> &mut ChunksByRelevance {
		&mut self.pending_chunks
	}

	pub fn chunk_relevance(&self) -> &relevancy::Relevance {
		&self.chunk_relevance
	}

	pub fn entity_relevance(&self) -> &relevancy::Relevance {
		&self.entity_relevance
	}

	pub fn send_entity_operations(
		&self,
		operations: Vec<(EntityOperation, hecs::Entity)>,
		serialized: &HashMap<hecs::Entity, binary::SerializedEntity>,
	) {
		use engine::channels::future::TrySendError;
		use replication::entity::Update;
		if let UpdateChannel::Remote(_, send_entities) = &self.channel {
			for (operation, entity) in operations.into_iter() {
				let update = match operation {
					EntityOperation::Relevant => {
						let serialized = serialized.get(&entity).unwrap();
						Update::Relevant(serialized.clone())
					}
					EntityOperation::Update => {
						let serialized = serialized.get(&entity).unwrap();
						Update::Update(serialized.clone())
					}
					EntityOperation::Irrelevant => Update::Irrelevant(entity),
					EntityOperation::Destroyed => Update::Destroyed(entity),
				};
				if let Err(err) = send_entities.try_send(update) {
					match err {
						TrySendError::Full(update) => {
							log::error!(target: &self.relevancy_log, "Failed to send entity update {:?}, unbounded async channel is full. This should never happen.", update);
						}
						TrySendError::Closed(update) => {
							log::error!(target: &self.relevancy_log, "Failed to send entity update {:?}, channel is closed. This should never happen because the channel can only be closed if the stream handle is dropped.", update);
						}
					}
				}
			}
		}
	}
}
