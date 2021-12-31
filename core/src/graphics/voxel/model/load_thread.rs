use crate::{
	app::state::ArcLockMachine,
	block::{self, Block},
	graphics::voxel::{atlas, camera, model, Face, RenderVoxel},
	network::storage::Storage,
};
use engine::{
	asset,
	graphics::{
		descriptor, flags, sampler,
		utility::{BuildFromDevice, NameableBuilder},
		ArcRenderChain, DescriptorCache, Texture,
	},
	task::{ArctexState, ScheduledTask},
	utility::{self, VoidResult},
};
use std::{
	collections::{HashMap, HashSet},
	pin::Pin,
	sync::{Arc, RwLock, Weak},
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
	pub fn start(
		app_state: &ArcLockMachine,
		storage: Weak<RwLock<Storage>>,
		render_chain: &ArcRenderChain,
		camera: &Arc<RwLock<camera::Camera>>,
	) -> Self {
		let state = ArctexState::default();

		let thread_state = state.clone();
		let thread_app_state = app_state.clone();
		let thread_storage = storage.clone();
		let thread_render_chain = render_chain.clone();
		let thread_camera = camera.clone();
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

			let mut block_ids = blocks
				.iter()
				.map(|(id, _block)| format!("{}", id))
				.collect::<Vec<_>>();
			block_ids.sort();
			log::debug!(target: LOG, "Block assets: [{}]", block_ids.join(", "));

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

			let mut cache_builder = model::Cache::builder();

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
			let mut atlas = atlas::Atlas::builder_2k();
			for (block_id, block) in blocks.iter() {
				let textures = block
					.textures()
					.iter()
					.map(|(_side, id)| (id, textures.get(&id).unwrap()))
					.collect::<HashMap<_, _>>();
				if !atlas.contains_or_fits_all(&textures) {
					log::error!(
						target: LOG,
						"Cannot fit textures for block {} in atlas",
						block_id
					);
					continue;
				}
				// Actually insert the textures
				atlas.insert_all(&textures)?;
			}

			log::debug!(target: LOG, "Creating block texture descriptor cache");
			let mut atlas_descriptor_cache = {
				let render_chain = thread_render_chain.read().unwrap();
				DescriptorCache::<(usize, usize)>::new(
					descriptor::layout::SetLayout::builder()
						.with_name("RenderVoxel.Atlas.DescriptorLayout")
						// In whatever set index a descriptor of this layout is bound to...
						// binding=0 is the texture sampler (for the atlas)
						.with_binding(
							0,
							flags::DescriptorKind::COMBINED_IMAGE_SAMPLER,
							1,
							flags::ShaderKind::Fragment,
						)
						.build(&render_chain.logical())?,
				)
			};

			// NOTE: Eventually blocks may want to specify their sampler by asset id.
			// When that becomes the case, we will need a dedicated sampler cache keyed by asset id.
			// For now, all blocks use the nearest-neighbor sampler.
			log::debug!(target: LOG, "Building atlas sampler");
			let atlas_sampler = Arc::new({
				let render_chain = thread_render_chain.read().unwrap();
				let max_anisotropy = render_chain.physical().max_sampler_anisotropy();
				sampler::Builder::default()
					.with_optname(Some("RenderVoxel.Atlas.Sampler".to_owned()))
					.with_magnification(flags::Filter::NEAREST)
					.with_minification(flags::Filter::NEAREST)
					.with_address_modes([flags::SamplerAddressMode::CLAMP_TO_EDGE; 3])
					.with_max_anisotropy(Some(max_anisotropy.min(16.0)))
					.with_border_color(flags::BorderColor::INT_OPAQUE_BLACK)
					.with_compare_op(Some(flags::CompareOp::ALWAYS))
					.with_mips(flags::SamplerMipmapMode::LINEAR, 0.0, 0.0..0.0)
					.build(&render_chain.logical())?
			});

			log::debug!(target: LOG, "Compiling atlas binary");
			let mut gpu_signals = Vec::new();
			let atlas = {
				let render_chain = thread_render_chain.read().unwrap();
				let (atlas, mut signals) =
					atlas.build(&render_chain, "RenderVoxel.Atlas.0".to_owned())?;
				gpu_signals.append(&mut signals);
				Arc::new(atlas)
			};

			// Create the descriptor set for the texture/atlas
			let descriptor_set = {
				use descriptor::update::*;
				let render_chain = thread_render_chain.read().unwrap();
				let descriptor_set = atlas_descriptor_cache.insert(
					(0, 0), // NOTE: This should be the id of the atlas and sampler in their respective caches
					Some(format!("RenderVoxel.Atlas.Descriptor({}, {})", 0, 0)),
					&render_chain,
				)?;

				Queue::default()
					.with(Operation::Write(WriteOp {
						destination: Descriptor {
							set: descriptor_set.clone(),
							binding_index: 0,
							array_element: 0,
						},
						kind: flags::DescriptorKind::COMBINED_IMAGE_SAMPLER,
						object: ObjectKind::Image(vec![ImageKind {
							view: atlas.view().clone(),
							sampler: atlas_sampler.clone(),
							layout: flags::ImageLayout::ShaderReadOnlyOptimal,
						}]),
					}))
					.apply(&render_chain.logical());

				descriptor_set
			};

			log::debug!(target: LOG, "Creating block models");
			let mut models = HashMap::new();
			for (block_id, block) in blocks.into_iter() {
				// Create the model for the block
				let mut builder = model::Model::builder();

				// Block models "own" the atlases. If no blocks reference the atlas, it is dropped.
				builder.set_atlas(atlas.clone(), atlas_sampler.clone(), descriptor_set.clone());

				for (side, texture_id) in block.textures() {
					for face in side.as_side_list().into_iter().map(|side| Face::from(side)) {
						let tex_coord = atlas.get(&texture_id).unwrap();
						builder.insert(face, tex_coord);
					}
				}

				models.insert(block_id, builder.build());
			}

			cache_builder.set_atlas_descriptor_cache(atlas_descriptor_cache);

			log::debug!(target: LOG, "Saving block models");
			// Move the block model data into the cache
			for (block_id, model) in models.into_iter() {
				let block_id = block::Lookup::lookup_value(&block_id).unwrap();
				cache_builder.insert(block_id, model);
			}

			log::debug!(target: LOG, "Finalizing model cache");
			let model_cache = {
				let render_chain = thread_render_chain.read().unwrap();
				let (model_cache, mut signals) = cache_builder.build(&render_chain)?;
				gpu_signals.append(&mut signals);
				model_cache
			};

			log::debug!(target: LOG, "Registering block renderer");
			RenderVoxel::add_state_listener(
				&thread_app_state,
				thread_storage.clone(),
				&thread_render_chain,
				&thread_camera,
				Arc::new(model_cache),
				gpu_signals,
			);

			thread_state.lock().unwrap().mark_complete();
			Ok(())
		});

		Self { state }
	}
}
