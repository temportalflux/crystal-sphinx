use crate::{
	app::state::ArcLockMachine,
	block::{self, Block},
	client::model::blender,
	common::network::Storage,
	graphics::voxel::{atlas, camera, model, RenderVoxel},
	CrystalSphinx,
};
use anyhow::Context;
use engine::{
	asset,
	graphics::{
		descriptor, flags,
		procedure::Phase,
		sampler,
		utility::{BuildFromDevice, NameableBuilder},
		Chain, DescriptorCache, Texture,
	},
	task,
	utility::ValueSet,
	Application,
};
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex, RwLock, Weak},
};

static LOG: &'static str = "model::loader";

/// Asynchronously loads the block assets so they can be loaded into the [`model cache`](super::Cache)
/// and stitched block texture asset.
pub fn load_models(
	systems: &Arc<ValueSet>,
	app_state: &ArcLockMachine,
	storage: Weak<RwLock<Storage>>,
	chain: &Arc<RwLock<Chain>>,
	phase: &Arc<Phase>,
	camera: &Arc<RwLock<camera::Camera>>,
	world: &Arc<RwLock<crate::entity::World>>,
) {
	let thread_systems = Arc::downgrade(&systems);

	let thread_app_state = app_state.clone();
	let thread_storage = storage.clone();
	let thread_chain = chain.clone();
	let thread_phase = Arc::downgrade(&phase);
	let thread_camera = camera.clone();
	let thread_world = Arc::downgrade(&world);
	task::spawn(LOG.to_string(), async move {
		//profiling::scope!("load_models");

		// Gather asset ids for all block assets
		let block_ids = match asset::Library::read().get_ids_of_type::<Block>() {
			Some(ids) => ids.clone(),
			None => return Ok(()), // No ids were scanned
		};

		// Load each block asset (synchronously)
		log::debug!(target: LOG, "Loading {} block assets", block_ids.len());
		let mut blocks = Vec::with_capacity(block_ids.len());
		let mut texture_ids = HashSet::with_capacity(blocks.len());
		// TODO: This should load all of the block assets at once so we aren't constantly opening the zip archive
		for asset_id in block_ids.into_iter() {
			let any_box = asset::Loader::load_sync(&asset_id)?;
			let block = match any_box.downcast::<Block>() {
				Ok(block) => block,
				_ => {
					log::error!(target: LOG, "Failed to interpret block asset {}", asset_id);
					return Ok(());
				}
			};
			for (entry, _faces) in block.textures().iter() {
				for texture_id in entry.texture_ids().iter() {
					texture_ids.insert(texture_id.clone());
				}
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
			let mut texture_map = HashMap::new();
			for (entry, _faces) in block.textures().iter() {
				for texture_id in entry.texture_ids().iter() {
					if let Some(texture) = textures.get(&texture_id) {
						texture_map.insert(texture_id, texture);
					} else {
						log::error!(
							target: LOG,
							"Failed to load texture {texture_id} for block {block_id}"
						);
					}
				}
			}
			if !atlas.contains_or_fits_all(&texture_map) {
				log::error!(
					target: LOG,
					"Cannot fit textures for block {} in atlas",
					block_id
				);
				continue;
			}
			// Actually insert the textures
			atlas.insert_all(&texture_map)?;
		}

		log::debug!(target: LOG, "Creating block texture descriptor cache");
		let mut atlas_descriptor_cache = {
			let chain = thread_chain.read().unwrap();
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
					.build(&chain.logical()?)?,
			)
		};

		// NOTE: Eventually blocks may want to specify their sampler by asset id.
		// When that becomes the case, we will need a dedicated sampler cache keyed by asset id.
		// For now, all blocks use the nearest-neighbor sampler.
		log::debug!(target: LOG, "Building atlas sampler");
		let atlas_sampler = Arc::new({
			let chain = thread_chain.read().unwrap();
			let max_anisotropy = chain.physical()?.max_sampler_anisotropy();
			sampler::Builder::default()
				.with_name("RenderVoxel.Atlas.Sampler".to_owned())
				.with_magnification(flags::Filter::NEAREST)
				.with_minification(flags::Filter::NEAREST)
				.with_address_modes([flags::SamplerAddressMode::CLAMP_TO_EDGE; 3])
				.with_max_anisotropy(Some(max_anisotropy.min(16.0)))
				.with_border_color(flags::BorderColor::INT_OPAQUE_BLACK)
				.with_compare_op(Some(flags::CompareOp::ALWAYS))
				.with_mips(flags::SamplerMipmapMode::LINEAR, 0.0, 0.0..0.0)
				.build(&chain.logical()?)?
		});

		log::debug!(target: LOG, "Compiling atlas binary");
		let atlas = {
			let chain = thread_chain.read().unwrap();
			Arc::new(atlas.build(
				&*chain,
				chain.signal_sender(),
				"RenderVoxel.Atlas.0".to_owned(),
			)?)
		};

		// Create the descriptor set for the texture/atlas
		let descriptor_set = {
			use descriptor::update::*;
			let chain = thread_chain.read().unwrap();
			let descriptor_set = atlas_descriptor_cache.insert(
				// NOTE: This should be the id of the atlas and sampler in their respective caches,
				// but right now there is only 1 atlas and 1 sampler
				(0, 0),
				format!("RenderVoxel.Atlas.Descriptor({}, {})", 0, 0),
				chain.persistent_descriptor_pool(),
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
				.apply(&*chain.logical()?);

			descriptor_set
		};

		log::debug!(target: LOG, "Creating block models");
		let mut models = HashMap::new();
		for (block_id, block) in blocks.into_iter() {
			// Create the model for the block
			let mut builder = model::Model::builder();

			builder.set_is_opaque(block.is_opaque());

			// Block models "own" the atlases. If no blocks reference the atlas, it is dropped.
			builder.set_atlas(atlas.clone(), atlas_sampler.clone(), descriptor_set.clone());

			if block.textures().is_empty() {
				log::warn!(target: LOG, "Block {} has no texture entries", block_id);
			}
			for (entry, faces) in block.textures() {
				let main_tex = match atlas.get(&entry.texture_id) {
					Some(tex) => tex,
					None => continue,
				};
				let biome_color_tex = entry
					.biome_color
					.1
					.as_ref()
					.map(|id| atlas.get(&id))
					.flatten();
				for face in faces.iter() {
					builder.insert(model::FaceData {
						main_tex,
						biome_color_tex,
						flags: model::Flags {
							face,
							biome_color_enabled: entry.biome_color.0,
							biome_color_masked: biome_color_tex.is_some(),
						},
					});
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
			let chain = thread_chain.read().unwrap();
			let model_cache = cache_builder
				.build(&*chain, chain.signal_sender())
				.context("build model cache")?;
			model_cache
		};

		// Gather asset ids for all model assets
		let blender_models = match asset::Library::read()
			.get_ids_of_type::<blender::Asset>()
			.cloned()
		{
			Some(model_ids) => {
				let mut models = HashMap::with_capacity(model_ids.len());
				// TODO: This should load all of the model assets at once so we aren't constantly opening the zip archive
				for asset_id in model_ids.into_iter() {
					let any_box =
						asset::Loader::load_sync(&asset_id).context("load blender model asset")?;
					let model = match any_box.downcast::<blender::Asset>() {
						Ok(model) => model,
						_ => {
							log::error!(
								target: LOG,
								"Failed to interpret model asset {}",
								asset_id
							);
							return Ok(());
						}
					};
					models.insert(asset_id, model.compiled().clone());
				}
				models
			}
			None => HashMap::new(),
		};
		let (blender_model_cache, mut texture_cache) = {
			let mut builder = blender::model::CacheBuilder::with_capacity(blender_models.len());
			for (id, model) in blender_models.into_iter() {
				builder.insert(id, model);
			}
			let chain = thread_chain.read().unwrap();
			let model_cache = Arc::new(
				builder
					.build(&*chain, "RenderEntity", chain.signal_sender())
					.context("build blender model cache")?,
			);
			let texture_cache = crate::client::model::texture::Cache::new(&*chain, atlas_sampler)
				.context("create blender texture cache")?;
			(model_cache, texture_cache)
		};
		texture_cache
			.load_default(
				CrystalSphinx::get_asset_id("entity/humanoid/textures/default"),
				thread_chain.clone(),
			)
			.await?;
		let texture_cache = Arc::new(Mutex::new(texture_cache));

		RenderVoxel::add_state_listener(
			&thread_app_state,
			thread_systems.clone(),
			thread_storage.clone(),
			Arc::downgrade(&thread_chain),
			thread_phase.clone(),
			Arc::downgrade(&thread_camera),
			Arc::new(model_cache),
		);
		let dependencies = crate::client::model::SystemDependencies {
			storage: thread_storage.clone(),
			render_chain: Arc::downgrade(&thread_chain),
			render_phase: thread_phase.clone(),
			camera: Arc::downgrade(&thread_camera),
			world: thread_world.clone(),
			blender_model_cache,
			texture_cache,
		};
		dependencies.add_state_listener(&thread_app_state);

		Ok(())
	});
}
