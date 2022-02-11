use std::sync::{Arc, RwLock, Weak};

use engine::network::stream::Registry;

use crate::{common::network::Storage, entity::system::replicator::relevancy::Relevance};

pub mod chunk;
pub mod relevancy;

pub fn register(registry: &mut Registry, storage: Weak<RwLock<Storage>>) {
	let local_relevance = Arc::new(RwLock::new(Relevance::default()));
	registry.register(relevancy::Builder {
		local_relevance: local_relevance.clone(),
		storage: storage.clone(),
	});
	registry.register(chunk::Builder {
		local_relevance: local_relevance.clone(),
		storage: storage.clone(),
	});
}
