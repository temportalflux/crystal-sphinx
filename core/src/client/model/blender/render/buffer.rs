use crate::client::model::blender::Model;
use engine::asset;
use std::collections::HashMap;

// TODO: Blender models need to be saved to a cache, which saves their data into vertex and index buffers.
// The cache should map the asset id to the vertex/index start offsets,
// and could in the future support updating models by id for hot-reloading.

// TODO: This is basically identical to voxel::model::cache (both the builder and the cache itself).
// 1. duplicate and adjust types/properties
// 2. abstract and unify pieces of it

/// Data manager and Vertex + Index buffer data for drawing static and rigged/unrigged models.
/// Exists for the lifetime of the application, not just during a gameplay session.
pub struct ModelBuffer {}

impl ModelBuffer {
	pub fn new() -> Self {
		Self {}
	}

	pub fn add_models(&mut self, models: HashMap<asset::Id, Model>) {}
}
