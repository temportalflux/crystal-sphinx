use super::{relevancy, EntityOperation};
use crate::{common::network::replication, entity::component::binary};
use engine::{
	math::nalgebra::Point3,
	network::socknet::{connection::Connection, stream, utility::JoinHandleList},
	task::JoinHandle,
	utility::Result,
};
use std::{
	collections::{HashMap, HashSet},
	net::SocketAddr,
	sync::{Arc, Weak},
};

/// Stateful information about what is relevant to a specific client.
///
/// Also servers as the connective tissue between the
/// [`replicator`](super::Replicator) system and the async task,
/// which dispatches entity replication data to a given client.
///
/// Its lifetime is owned by the replicator system.
pub struct Handle {
	send_world_rel: async_channel::Sender<(relevancy::Relevance, Option<HashSet<Point3<i64>>>)>,
	send_entities: async_channel::Sender<replication::entity::Update>,
	chunk_relevance: relevancy::Relevance,
	entity_relevance: relevancy::Relevance,
	relevancy_log: String,
}

impl Handle {
	pub fn new(address: SocketAddr, connection: &Weak<Connection>) -> Self {
		let relevancy_log = format!("relevancy[{}]", address);
		let (send_world_rel, recv_world_rel) = async_channel::unbounded();
		let (send_entities, recv_entities) = async_channel::unbounded();

		{
			use replication::entity::send::Handler;
			Handler::spawn(connection.clone(), recv_entities);
		}

		{
			use replication::world::relevancy::Handler;
			Handler::spawn(connection.clone(), recv_world_rel);
		}

		Self {
			send_world_rel,
			send_entities,
			chunk_relevance: relevancy::Relevance::default(),
			entity_relevance: relevancy::Relevance::default(),
			relevancy_log,
		}
	}

	pub fn set_chunk_relevance(&mut self, relevance: relevancy::Relevance, new_chunks: Option<HashSet<Point3<i64>>>) {
		if relevance != self.chunk_relevance {
			use async_channel::TrySendError;

			self.chunk_relevance = relevance;

			if let Err(err) = self.send_world_rel.try_send((
				self.chunk_relevance.clone(),
				new_chunks,
			)) {
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

	}

	pub fn chunk_relevance(&self) -> &relevancy::Relevance {
		&self.chunk_relevance
	}

	pub fn set_entity_relevance(&mut self, relevance: relevancy::Relevance) {
		self.entity_relevance = relevance;
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
