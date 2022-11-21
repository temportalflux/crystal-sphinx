//! Stream for replicating data about the physical world; what chunks are relevant and what blocks each chunk contains.
//!
//! See [`register`] for stream graph.
use std::sync::{Arc, RwLock, Weak};

use engine::{
	channels::future::{Receiver, Sender},
	utility::ValueSet,
};
use socknet::stream::Registry;

use crate::{
	common::world,
	entity::system::replicator::relevancy::{Relevance, WorldUpdate},
	server::world::chunk::Chunk,
};

pub mod chunk;
pub mod relevancy;

/// Async channel for sending world updates to the world-relevancy async task.
pub type SendUpdate = Sender<WorldUpdate>;
/// Async channel for receiving world updates in the world-relevancy async task.
pub type RecvUpdate = Receiver<WorldUpdate>;

/// Async channel for sending chunks to one of the chunk replication async tasks.
pub type SendChunks = Sender<Weak<RwLock<Chunk>>>;
/// Async channel for receiving chunks in one of the chunk replication async tasks.
pub type RecvChunks = Receiver<Weak<RwLock<Chunk>>>;

#[cfg_attr(doc, aquamarine::aquamarine)]
/// Client-Initiated stream which handles the authentication protocol.
/// While clients are technically connected when the stream is initiated,
/// they don't really count as valid clients until the stream is concluded.
///
/// [Edit Diagram](https://mermaid.live/edit#pako:eNptkcFqwzAMhl9F-LRD9gI5FEZW2GmM5eqLamuLiSNnthwope8-uc1gsPliW_-nn1_2xbjkyfSm0FcldvQc8DPjYtkKVklclxPldlsxS3BhRRYYAQuMlLe_0tCkIQZiadJrEoKkHIw9vJOjsJHfdXiqMukeHCp03FrpoRDBC7IvE87UHMbHw2Ho4VgETzGUSV0ibcjuDEUy3aPGlFYYpsrzeKu9pRRb_Z_2G7W3AjeI2FsGXb_Cdtpztyow0yqAUZND1bQR3D2-D8UlZnJSTGcWygsGr095aW7W6GwLWdPr0WOerbF8Va6uXsc9-iApm_4DY6HOtKcez-xML7nSD7T_xU5dvwE3YJpC)
/// ```mermaid
/// sequenceDiagram
/// 	autonumber
/// 	participant S as Server
/// 	participant C as Client
/// 	Note over S: Received Client Authenticate Event (see Handshake)
/// 	S->>C: Establish Relevancy stream
/// 	loop ChunkStreamPool
/// 		S->>C: Establish Chunk stream n
/// 	end
/// 	Note over S,C: Streams kept alive until client disconnects
/// ```
pub fn register(registry: &mut Registry, systems: &Arc<ValueSet>) {
	let storage = systems
		.get_arclock::<crate::common::network::Storage>()
		.unwrap();
	let database = systems.get_arclock::<world::Database>().unwrap();
	let chunk_channel = systems.get_arc::<crate::client::world::ChunkChannel>();

	let local_relevance = Arc::new(RwLock::new(Relevance::default()));
	registry.register(relevancy::Identifier {
		server: Arc::default(),
		client: Arc::new(relevancy::client::AppContext {
			local_relevance: local_relevance.clone(),
			storage: Arc::downgrade(&storage),
			chunk_channel: chunk_channel.map(|arc| Arc::downgrade(&arc)),
		}),
	});
	registry.register(chunk::Identifier {
		server: Arc::default(),
		client: Arc::new(chunk::client::AppContext {
			local_relevance: local_relevance.clone(),
			storage: Arc::downgrade(&storage),
			database: Arc::downgrade(&database),
		}),
	});
}
