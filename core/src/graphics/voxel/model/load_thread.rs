use crate::{block::Block, graphics::voxel::model};
use engine::{
	asset,
	graphics::Texture,
	task::{ArctexState, ScheduledTask},
	utility::{self, VoidResult},
};
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	task::{Context, Poll},
};

static LOG: &'static str = "model::loader";

/// Loads the block assets so they can be loaded into the [`model cache`](super::Cache)
/// and stitched block texture asset.
pub struct Load {
	state: ArctexState,
}

impl ScheduledTask for Load {
	fn state(&self) -> &ArctexState {
		&self.state
	}
}
impl futures::future::Future for Load {
	type Output = ();
	fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
		self.poll_state(ctx)
	}
}

impl Load {
	pub fn start(model_cache: &model::ArcLockCache) -> Self {
		let state = ArctexState::default();

		let thread_state = state.clone();
		let thread_model_cache = model_cache.clone();
		utility::spawn_thread(LOG, move || -> VoidResult {
			// Gather asset ids for all block assets
			let block_ids = match asset::Library::read().get_ids_of_type::<Block>() {
				Some(ids) => ids.clone(),
				None => return Ok(()), // No ids were scanned
			};

			// Load each block asset (synchronously)
			log::debug!(target: LOG, "Loading {} block assets", block_ids.len());
			let mut blocks = Vec::with_capacity(block_ids.len());
			let mut texture_ids = HashSet::with_capacity(blocks.len());
			for asset_id in block_ids.into_iter() {
				let any_box = asset::Loader::load_sync(&asset_id)?;
				let block = match any_box.downcast::<Block>() {
					Ok(block) => block,
					_ => {
						log::error!(target: LOG, "Failed to interpret block asset {}", asset_id);
						return Ok(());
					}
				};
				for (_side, id) in block.textures().iter() {
					texture_ids.insert(id.clone());
				}
				blocks.push((asset_id, block));
			}

			// Load all block textures
			log::debug!(
				target: LOG,
				"Loading {} block texture assets",
				texture_ids.len()
			);
			let mut textures = HashMap::with_capacity(texture_ids.len());
			for asset_id in texture_ids.into_iter() {
				if let Ok(any_box) = asset::Loader::load_sync(&asset_id) {
					if let Ok(texture) = any_box.downcast::<Texture>() {
						textures.insert(asset_id, texture);
					}
				}
			}

			let mut model_cache = thread_model_cache.write().unwrap();

			// The textures for each block are now loaded.
			// Now they must be stitched into textures such that
			// each block only needs to bind 1 atlas.
			//
			// NOTE:
			// We are only using a 2k texture right now (2048x2048)
			// and expect all block textures to be 16x16.
			// If/when we support textures larger than 16x16
			// OR we exceed 16,384 16x16 textures, we will need to do more complex
			// calculations such that all the textures fit onto atlases
			// and each block only needs access to 1 atlas
			// (even if it means uploading a given block texture on multiple atlases).
			log::debug!(target: LOG, "Stitching block textures");
			for (_block_id, block) in blocks.iter() {
				let atlas = model_cache.atlas_mut();
				let textures = block
					.textures()
					.iter()
					.map(|(_side, id)| (id, textures.get(&id).unwrap()))
					.collect::<HashMap<_, _>>();
				if !atlas.contains_or_fits_all(&textures) {
					atlas.insert_all(&textures)?;
				}
			}

			log::debug!(target: LOG, "Compiling atlas binary");
			let _binary = model_cache.atlas_mut().as_binary();

			// TODO: Save atlas texture graphics arc and the texture
			//			 coordinates to models, which are inserted into the model cache

			thread_state.lock().unwrap().mark_complete();
			Ok(())
		});

		Self { state }
	}
}
