use anyhow::Context;
use engine::{
	asset, channels,
	graphics::{
		command, descriptor, flags, image,
		image_view::{self, View},
		sampler::Sampler,
		structs,
		utility::{BuildFromDevice, NameableBuilder, NamedObject},
		Chain, DescriptorCache, GpuOpContext, GpuOperationBuilder, Texture,
	},
	math::nalgebra::Vector2,
};
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, RwLock, Weak},
};

// TODO: Currently, once a texture is loaded, it is kept around permanently until cache is dropped. In the future,
// we need to specify whether a texture is permanent (e.g. default humanoid) or keep-until-no-longer-used (e.g. player textures).

pub struct Cache {
	descriptors: DescriptorCache<asset::Id>,
	sampler: Arc<Sampler>,
	texture_loaded: channels::future::Pair<(asset::Id, Vector2<usize>, Vec<u8>)>,
	ids_in_async_load: HashSet<asset::Id>,
	loaded_textures: HashMap<asset::Id, Arc<image_view::View>>,
	default_id: Option<asset::Id>,
}

impl Cache {
	pub fn new(chain: &Chain, sampler: Arc<Sampler>) -> anyhow::Result<Self> {
		let descriptors = DescriptorCache::<asset::Id>::new(
			descriptor::layout::SetLayout::builder()
				.with_name("RenderModel.DescriptorLayout")
				// In whatever set index a descriptor of this layout is bound to...
				// binding=0 is the texture sampler (for the atlas)
				.with_binding(
					0,
					flags::DescriptorKind::COMBINED_IMAGE_SAMPLER,
					1, // TODO: This may need to be much larger
					flags::ShaderKind::Fragment,
				)
				.build(&chain.logical()?)?,
		);
		Ok(Self {
			descriptors,
			sampler,
			texture_loaded: channels::future::unbounded(),
			ids_in_async_load: HashSet::new(),
			loaded_textures: HashMap::new(),
			default_id: None,
		})
	}

	pub fn descriptor_layout(&self) -> &Arc<descriptor::layout::SetLayout> {
		&self.descriptors.layout()
	}

	pub async fn load_default(
		&mut self,
		id: asset::Id,
		chain: Arc<RwLock<Chain>>,
	) -> anyhow::Result<()> {
		self.load(id.clone(), chain).await?;
		self.default_id = Some(id);
		Ok(())
	}

	/// Load a specific texture into memory
	pub async fn load(&mut self, id: asset::Id, chain: Arc<RwLock<Chain>>) -> anyhow::Result<()> {
		if self.has_loaded(&id) {
			return Ok(());
		}
		let (size, binary) = Self::load_texture(&id).await?;
		{
			let chain = chain.read().unwrap();
			let view =
				Self::create_texture(&*chain, chain.signal_sender(), id.as_string(), size, binary)?;
			self.insert(&*chain, id, view)?;
		}
		Ok(())
	}

	/// Marks a texture for loading.
	pub fn mark_required(&mut self, id: &asset::Id) {
		if self.has_loaded(id) {
			return;
		}

		self.ids_in_async_load.insert(id.clone());

		let id = id.clone();
		let sender = self.texture_loaded.0.clone();
		engine::task::spawn("render-model-texture-loader".to_string(), async move {
			let (size, binary) = Self::load_texture(&id).await?;
			sender.send((id, size, binary)).await?;
			Ok(())
		});
	}

	async fn load_texture(id: &asset::Id) -> anyhow::Result<(Vector2<usize>, Vec<u8>)> {
		let mut asset = asset::Loader::load_t::<Texture>(id)
			.await
			.context("loading texture for entity model")?;
		Ok(asset.take_data().unwrap())
	}

	pub fn load_pending(&mut self, chain: &Chain) -> anyhow::Result<()> {
		while let Ok((id, size, binary)) = self.texture_loaded.1.try_recv() {
			self.ids_in_async_load.remove(&id);
			let view =
				Self::create_texture(chain, chain.signal_sender(), id.as_string(), size, binary)?;
			self.insert(&*chain, id, view)?;
		}
		Ok(())
	}

	fn create_texture(
		context: &impl GpuOpContext,
		signal_sender: &channels::mpsc::Sender<Arc<command::Semaphore>>,
		name: String,
		size: Vector2<usize>,
		binary: Vec<u8>,
	) -> anyhow::Result<Arc<image_view::View>> {
		let image = Arc::new(image::Image::create_gpu(
			&context.object_allocator()?,
			name.clone(),
			flags::format::SRGB_8BIT,
			structs::Extent3D {
				width: size.x as u32,
				height: size.y as u32,
				depth: 1,
			},
		)?);

		GpuOperationBuilder::new(format!("Create({})", image.name()), context)?
			.begin()?
			.format_image_for_write(&image)
			.stage(&binary[..])?
			.copy_stage_to_image(&image)
			.format_image_for_read(&image)
			.send_signal_to(signal_sender)?
			.end()?;

		let view = Arc::new(
			image_view::View::builder()
				.with_name(format!("{}.View", name))
				.for_image(image)
				.with_view_type(flags::ImageViewType::TYPE_2D)
				.with_range(
					structs::subresource::Range::default().with_aspect(flags::ImageAspect::COLOR),
				)
				.build(&context.logical_device()?)?,
		);

		Ok(view)
	}

	fn has_loaded(&self, id: &asset::Id) -> bool {
		self.ids_in_async_load.contains(id) || self.loaded_textures.contains_key(id)
	}

	pub fn insert(&mut self, chain: &Chain, id: asset::Id, view: Arc<View>) -> anyhow::Result<()> {
		use descriptor::update::*;

		let name = format!("RenderModel.Descriptor({id})");
		let descriptor_set =
			self.descriptors
				.insert(id.clone(), name, chain.persistent_descriptor_pool())?;

		Queue::default()
			.with(Operation::Write(WriteOp {
				destination: Descriptor {
					set: descriptor_set.clone(),
					binding_index: 0,
					array_element: 0,
				},
				kind: flags::DescriptorKind::COMBINED_IMAGE_SAMPLER,
				object: ObjectKind::Image(vec![ImageKind {
					view: view.clone(),
					sampler: self.sampler.clone(),
					layout: flags::ImageLayout::ShaderReadOnlyOptimal,
				}]),
			}))
			.apply(&*chain.logical()?);

		self.loaded_textures.insert(id, view);

		Ok(())
	}

	pub fn get(&self, id: &asset::Id) -> Option<&Weak<descriptor::Set>> {
		self.descriptors.get(id)
	}

	pub fn get_or_default(&self, id: &asset::Id) -> Option<&Weak<descriptor::Set>> {
		if let Some(descriptor) = self.get(id) {
			return Some(descriptor);
		}

		if let Some(id) = &self.default_id {
			return self.get(id);
		}

		None
	}
}
