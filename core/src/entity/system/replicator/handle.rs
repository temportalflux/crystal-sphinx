use super::{relevancy, EntityOperation};
use crate::{common::network::replication, entity::component::binary};
use engine::{math::nalgebra::Point3, network::socknet::connection::Connection};
use std::{
	collections::{HashMap, HashSet},
	net::SocketAddr,
	sync::Weak,
};

/// Stateful information about what is relevant to a specific client.
///
/// Also servers as the connective tissue between the
/// [`replicator`](super::Replicator) system and the async task,
/// which dispatches entity replication data to a given client.
///
/// Its lifetime is owned by the replicator system.
pub struct Handle {
	send_world_rel: async_channel::Sender<relevancy::WorldUpdate>,
	send_entities: async_channel::Sender<replication::entity::Update>,
	chunk_relevance: relevancy::Relevance,
	entity_relevance: relevancy::Relevance,
	relevancy_log: String,
	pending_chunks: HashSet<Point3<i64>>,
}

impl Handle {
	pub fn new(address: SocketAddr, connection: &Weak<Connection>) -> Self {
		let relevancy_log = format!("relevancy[{}]", address);
		let (send_world_rel, recv_world_rel) = async_channel::unbounded();
		let (send_entities, recv_entities) = async_channel::unbounded();
		let (send_chunks, recv_chunks) = async_channel::unbounded();

		{
			use replication::entity::send::Handler;
			Handler::spawn(connection.clone(), recv_entities);
		}

		{
			use replication::world::relevancy::Handler;
			Handler::spawn(connection.clone(), recv_world_rel, send_chunks);
		}

		for i in 0..10 {
			use replication::world::chunk::server;
			server::Chunk::spawn(connection.clone(), i, recv_chunks.clone());
		}

		Self {
			send_world_rel,
			send_entities,
			chunk_relevance: relevancy::Relevance::default(),
			entity_relevance: relevancy::Relevance::default(),
			relevancy_log,
			pending_chunks: HashSet::new(),
		}
	}

	pub fn send_relevance_updates(&mut self, updates: Vec<relevancy::Update>) {
		for update in updates.into_iter() {
			match update {
				relevancy::Update::World(update) => {
					if let relevancy::WorldUpdate::Relevance(relevance) = &update {
						if *relevance == self.chunk_relevance {
							continue;
						}
						self.chunk_relevance = relevance.clone();
					}
					self.send_world_update(update);
				}
				relevancy::Update::Entity(relevance) => {
					self.entity_relevance = relevance;
				}
			}
		}
	}

	pub fn send_world_update(&mut self, update: relevancy::WorldUpdate) {
		use async_channel::TrySendError;
		if let Err(err) = self.send_world_rel.try_send(update) {
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

	pub fn take_pending_chunks(&mut self) -> HashSet<Point3<i64>> {
		self.pending_chunks.drain().collect()
	}

	pub fn insert_pending_chunk(&mut self, coordinate: Point3<i64>) {
		self.pending_chunks.insert(coordinate);
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
		use async_channel::TrySendError;
		use replication::entity::Update;
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
			if let Err(err) = self.send_entities.try_send(update) {
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
