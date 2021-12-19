use std::sync::{Arc, RwLock};

pub mod archetype;
pub mod component;

pub use hecs::World;
/// Alias for Arc<RwLock<[`World`](hecs::World)>>
pub type ArcLockEntityWorld = Arc<RwLock<World>>;
