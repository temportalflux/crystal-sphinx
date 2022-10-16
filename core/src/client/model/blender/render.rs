use crate::{
	client::model::{
		blender::model,
		instance::{self, Instance},
		texture, DescriptorId,
	},
	graphics::{model::Model as ModelTrait, voxel::camera},
	CrystalSphinx,
};
use anyhow::Result;
use engine::{
	graphics::{
		self,
		chain::{operation::RequiresRecording, Operation},
		command, flags,
		procedure::Phase,
		resource::ColorBuffer,
		Chain, Drawable, Uniform,
	},
	Application,
};
use std::sync::{Arc, Mutex, RwLock};

static ID: &'static str = "render-entity";

/// Management of non-block models and executing draw-calls for entities during frame render.
/// Exists only as long as the user is in a world
/// (it is saved to session storage, created when entering a game and destroyed upon leaving).
pub struct RenderModel {
	drawable: Drawable,
	camera_uniform: Uniform,
	camera: Arc<RwLock<camera::Camera>>,
	model_cache: Arc<model::Cache>,
	instance_buffer: Arc<RwLock<instance::Buffer>>,
	texture_cache: Arc<Mutex<texture::Cache>>,
}

impl RenderModel {
	pub fn create(
		chain: &Arc<RwLock<Chain>>,
		phase: &Arc<Phase>,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		instance_buffer: Arc<RwLock<instance::Buffer>>,
		texture_cache: Arc<Mutex<texture::Cache>>,
	) -> Result<Arc<RwLock<Self>>> {
		log::info!(target: ID, "Initializing");
		let instance = Self::new(
			&chain.read().unwrap(),
			camera,
			model_cache,
			instance_buffer,
			texture_cache,
		)?
		.arclocked();

		log::trace!(target: ID, "Adding to render chain");
		let mut chain = chain.write().unwrap();
		// priority is 1 so that its sorted AFTER `RenderVoxel` which is priority 0
		chain.add_operation(phase, Arc::downgrade(&instance), Some(1))?;

		Ok(instance)
	}

	fn new(
		chain: &Chain,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		instance_buffer: Arc<RwLock<instance::Buffer>>,
		texture_cache: Arc<Mutex<texture::Cache>>,
	) -> Result<Self> {
		log::trace!(target: ID, "Creating renderer");

		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/entity/vertex"))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/entity/fragment"))?;

		let camera_uniform = Uniform::new::<camera::UniformData, &str>(
			"RenderEntity.Camera",
			&chain.logical()?,
			&chain.allocator()?,
			chain.persistent_descriptor_pool(),
			chain.view_count(),
		)?;

		Ok(Self {
			drawable,
			camera_uniform,
			camera,
			model_cache,
			instance_buffer,
			texture_cache,
		})
	}

	fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl Operation for RenderModel {
	fn initialize(&mut self, chain: &Chain) -> anyhow::Result<()> {
		self.drawable.create_shaders(&chain.logical()?)?;
		self.camera_uniform
			.write_descriptor_sets(&*chain.logical()?);
		Ok(())
	}

	fn construct(&mut self, chain: &Chain, subpass_index: usize) -> anyhow::Result<()> {
		use graphics::pipeline::{state::*, Pipeline};

		let sample_count = {
			let arc = chain.resources().get::<ColorBuffer>()?;
			let color_buffer = arc.read().unwrap();
			color_buffer.sample_count()
		};

		let tex_desc_layout = self
			.texture_cache
			.lock()
			.unwrap()
			.descriptor_layout()
			.clone();

		self.drawable.create_pipeline(
			&chain.logical()?,
			vec![self.camera_uniform.layout(), &tex_desc_layout],
			Pipeline::builder()
				.with_vertex_layout(
					vertex::Layout::default()
						.with_object::<model::Vertex>(0, flags::VertexInputRate::VERTEX)
						.with_object::<Instance>(1, flags::VertexInputRate::INSTANCE),
				)
				.set_viewport_state(Viewport::from(*chain.extent()))
				.set_color_blending(
					color_blend::ColorBlend::default()
						.add_attachment(color_blend::Attachment::default()),
				)
				.with_multisampling(
					Multisampling::default()
						.with_sample_count(sample_count)
						.with_sample_shading(Some(0.25)),
				)
				.with_depth_stencil(
					DepthStencil::default()
						.with_depth_test()
						.with_depth_write()
						.with_depth_bounds(0.0, 1.0)
						.with_depth_compare_op(flags::CompareOp::LESS),
				),
			chain.render_pass(),
			subpass_index,
		)?;
		Ok(())
	}

	fn deconstruct(&mut self, _chain: &Chain) -> anyhow::Result<()> {
		self.drawable.destroy_pipeline()?;
		Ok(())
	}

	fn prepare_for_frame(&mut self, _chain: &Chain) -> anyhow::Result<()> {
		Ok(())
	}

	fn prepare_for_submit(
		&mut self,
		chain: &Chain,
		frame_image: usize,
	) -> anyhow::Result<RequiresRecording> {
		let data = self
			.camera
			.read()
			.unwrap()
			.as_uniform_data(&chain.resolution());
		self.camera_uniform.write_data(frame_image, &data)?;

		if let Ok(mut cache) = self.texture_cache.lock() {
			cache.load_pending(chain)?;
		}

		// TODO: There should probably be separate instance buffers for each frame (ring of 3),
		// so that updating one buffer doesn't wait for the previous from to be complete.
		// If the instances change, we need to re-record the render.
		let was_changed = match self.instance_buffer.write() {
			Ok(mut buffer) => buffer.submit(chain, chain.signal_sender())?,
			Err(_) => false,
		};
		Ok(match was_changed {
			true => RequiresRecording::CurrentFrame,
			false => RequiresRecording::NotRequired,
		})
	}

	fn record(&mut self, buffer: &mut command::Buffer, buffer_index: usize) -> anyhow::Result<()> {
		use graphics::debug;
		profiling::scope!("record:RenderModel");

		buffer.begin_label("Draw:Model", debug::LABEL_COLOR_DRAW);
		{
			self.drawable.bind_pipeline(buffer);

			// TODO: This is highly inefficient. Draw calls should be grouped by model and then texture.
			let (instance_buffer, instance_ids) = {
				let instances = self.instance_buffer.read().unwrap();
				let buffer = instances.buffer().clone();
				let ids = instances.submitted().clone();
				(buffer, ids)
			};

			buffer.bind_vertex_buffers(0, vec![&self.model_cache.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&instance_buffer], vec![0]);
			buffer.bind_index_buffer(&self.model_cache.index_buffer, 0);

			let mut bindings = Vec::with_capacity(instance_ids.len());
			for (
				_entity,
				DescriptorId {
					model_id,
					texture_id,
				},
				instance_idx,
			) in instance_ids.into_iter()
			{
				let (model, index_start, vertex_offset) = match self.model_cache.get(&model_id) {
					Some(entry) => entry,
					None => continue,
				};
				let texture_descriptor_set = {
					let texture_cache = self.texture_cache.lock().unwrap();
					texture_cache.get_or_default(&texture_id).cloned()
				};
				let tex_desc = match texture_descriptor_set {
					Some(set) => set.upgrade().unwrap(),
					None => continue,
				};
				let label = format!("Draw:Model({model_id}, {texture_id})");
				bindings.push((
					label,
					model.indices().len(),
					*index_start,
					*vertex_offset,
					instance_idx,
					tex_desc,
				));
			}

			for (label, index_count, index_start, vertex_offset, instance_idx, tex_desc_set) in
				bindings.into_iter()
			{
				buffer.begin_label(label, debug::LABEL_COLOR_DRAW);

				// Bind the texture-atlas and camera descriptors
				self.drawable.bind_descriptors(
					buffer,
					vec![
						// Descriptor set=0 is the camera uniform.
						// layout(set = 0, binding = 0) uniform CameraUniform ...
						&self.camera_uniform.get_set(buffer_index).unwrap(),
						// Descriptor set=1 is the texture sampler.
						// The binding number is defined when creating the descriptor cache.
						// layout(set = 1, binding = 0) uniform sampler2D texSampler;
						&tex_desc_set,
					],
				);

				buffer.draw(index_count, index_start, 1, instance_idx, vertex_offset);

				buffer.end_label();
			}
		}
		buffer.end_label();

		Ok(())
	}
}
