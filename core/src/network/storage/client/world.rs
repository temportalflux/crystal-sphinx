use std::sync::{Arc, RwLock};

pub type ArcLockWorld = Arc<RwLock<World>>;
/// Contains data pertaining to how a world is presented when a client is connected to a game server.
pub struct World {}
