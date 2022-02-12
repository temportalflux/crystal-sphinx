use std::sync::{Arc, RwLock, Weak};

use socknet::stream::Registry;

use crate::{
	common::network::Storage,
	entity::system::replicator::relevancy::{Relevance, WorldUpdate},
	server::world::chunk::Chunk,
};

pub mod chunk;
pub mod relevancy;

pub type SendUpdate = async_channel::Sender<WorldUpdate>;
pub type RecvUpdate = async_channel::Receiver<WorldUpdate>;

pub type SendChunks = async_channel::Sender<Weak<RwLock<Chunk>>>;
pub type RecvChunks = async_channel::Receiver<Weak<RwLock<Chunk>>>;

pub fn register(registry: &mut Registry, storage: Weak<RwLock<Storage>>) {
	let local_relevance = Arc::new(RwLock::new(Relevance::default()));
	registry.register(relevancy::Identifier {
		server: Arc::default(),
		client: Arc::new(relevancy::client::AppContext {
			local_relevance: local_relevance.clone(),
			storage: storage.clone(),
		}),
	});
	registry.register(chunk::Identifier {
		server: Arc::default(),
		client: Arc::new(chunk::client::AppContext {
			local_relevance: local_relevance.clone(),
			storage: storage.clone(),
		}),
	});
}
