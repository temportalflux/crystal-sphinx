use std::sync::{Arc, RwLock, Weak};

use engine::{network::socknet::stream, utility::Result};

use crate::{entity::system::replicator::relevancy::Relevance, network::storage::Storage};

pub mod client;
pub mod server;

pub struct Builder {
	pub local_relevance: Arc<RwLock<Relevance>>,
	pub storage: Weak<RwLock<Storage>>,
}

impl stream::Identifier for Builder {
	fn unique_id() -> &'static str {
		"replication::world::chunk"
	}
}

impl stream::send::Builder for Builder {
	type Opener = stream::uni::Opener;
}

impl stream::recv::Builder for Builder {
	type Extractor = stream::uni::Extractor;
	type Receiver = client::Chunk;
}

impl Builder {
	pub fn client_chunk_cache(&self) -> Result<crate::client::world::chunk::cache::ArcLock> {
		use crate::network::storage::Error::{
			FailedToReadClient, FailedToReadStorage, InvalidClient, InvalidStorage,
		};
		let arc_storage = self.storage.upgrade().ok_or(InvalidStorage)?;
		let storage = arc_storage.read().map_err(|_| FailedToReadStorage)?;
		let arc = storage.client().as_ref().ok_or(InvalidClient)?;
		let client = arc.read().map_err(|_| FailedToReadClient)?;
		Ok(client.chunk_cache().clone())
	}
}
