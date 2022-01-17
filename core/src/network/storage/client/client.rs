use crate::world::chunk::cache::client::{ArcLockClientCache, ClientCache};
use std::sync::{Arc, RwLock};

pub type ArcLockClient = Arc<RwLock<Client>>;
/// Container class for all client data which is present when a user is connected to a game server.
pub struct Client {
	chunk_cache: ArcLockClientCache,
}

impl Default for Client {
	fn default() -> Self {
		let chunk_cache = Arc::new(RwLock::new(ClientCache::new()));
		Self { chunk_cache }
	}
}

impl Client {
	pub fn chunk_cache(&self) -> &ArcLockClientCache {
		&self.chunk_cache
	}
}
