use crate::{
	app::state::{self, ArcLockMachine},
	block,
	client::world::chunk,
	common::network::Storage,
	graphics::voxel::{
		camera,
		instance::{self, Instance},
		model,
	},
	CrystalSphinx,
};
use anyhow::Result;
use engine::{
	asset,
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
use std::sync::{Arc, RwLock, Weak};

static ID: &'static str = "render-voxel";

pub type ArcLockRenderVoxel = Arc<RwLock<RenderVoxel>>;
pub struct RenderVoxel {
	drawable: Drawable,
	instance_buffer: instance::Buffer,
	camera_uniform: Uniform,
	camera: Arc<RwLock<camera::Camera>>,
	model_cache: Arc<model::Cache>,
}

impl RenderVoxel {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/subpass/world")
	}

	pub fn add_state_listener(
		app_state: &ArcLockMachine,
		storage: Weak<RwLock<Storage>>,
		chain: Weak<RwLock<Chain>>,
		phase: Weak<Phase>,
		camera: Weak<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_chain = chain;
		let callback_phase = phase;
		let callback_storage = storage;
		let callback_model_cache = model_cache;
		let callback_camera = camera;
		Storage::<ArcLockRenderVoxel>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				profiling::scope!("init-render", ID);
				log::trace!(target: ID, "Received Enter(InGame) transition");
				let chain = callback_chain.upgrade().unwrap();
				let phase = callback_phase.upgrade().unwrap();
				let arc_camera = callback_camera.upgrade().unwrap();

				let chunk_receiver = match callback_storage.upgrade() {
					Some(arc_storage) => {
						let storage = arc_storage.read().unwrap();
						match storage.client() {
							Some(arc_client) => {
								let client = arc_client.read().unwrap();
								client.chunk_receiver().clone()
							}
							None => {
								log::error!(target: ID, "Failed to find client storage");
								return Ok(None);
							}
						}
					}
					None => {
						log::error!(target: ID, "Failed to find storage");
						return Ok(None);
					}
				};

				Ok(
					match Self::create(
						&chain,
						&phase,
						arc_camera,
						callback_model_cache.clone(),
						chunk_receiver,
					) {
						Ok(arclocked) => Some(arclocked),
						Err(err) => {
							log::error!(target: ID, "{}", err);
							None
						}
					},
				)
			});
	}

	fn create(
		chain: &Arc<RwLock<Chain>>,
		phase: &Arc<Phase>,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		chunk_receiver: chunk::OperationReceiver,
	) -> Result<ArcLockRenderVoxel> {
		log::info!(target: ID, "Initializing");
		let render_chunks =
			Self::new(&chain.read().unwrap(), camera, model_cache, chunk_receiver)?.arclocked();

		log::trace!(target: ID, "Adding to render chain");
		let mut chain = chain.write().unwrap();
		chain.add_operation(phase, Arc::downgrade(&render_chunks))?;

		Ok(render_chunks)
	}

	fn new(
		chain: &Chain,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		chunk_receiver: chunk::OperationReceiver,
	) -> Result<Self> {
		log::trace!(target: ID, "Creating renderer");

		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/vertex"))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/fragment"))?;

		let instance_buffer = instance::Buffer::new(
			&chain.allocator()?,
			Arc::downgrade(&model_cache),
			chunk_receiver,
		)?;

		let camera_uniform = Uniform::new::<camera::UniformData, &str>(
			"RenderVoxel.Camera",
			&chain.logical()?,
			&chain.allocator()?,
			chain.persistent_descriptor_pool(),
			chain.view_count(),
		)?;

		Ok(Self {
			drawable,
			instance_buffer,
			camera_uniform,
			camera,
			model_cache,
		})
	}

	fn arclocked(self) -> ArcLockRenderVoxel {
		Arc::new(RwLock::new(self))
	}
}

impl Drop for RenderVoxel {
	fn drop(&mut self) {
		log::info!(target: ID, "Dropping from subpass({}).", Self::subpass_id());
	}
}

impl Operation for RenderVoxel {
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

		self.drawable.create_pipeline(
			&chain.logical()?,
			vec![
				self.camera_uniform.layout(),
				self.model_cache.descriptor_layout(),
			],
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

		// If the instances change, we need to re-record the render
		let was_changed = self.instance_buffer.submit_pending_changes(&chain)?;
		Ok(match was_changed {
			true => RequiresRecording::CurrentFrame,
			false => RequiresRecording::NotRequired,
		})
	}

	fn record(&mut self, buffer: &mut command::Buffer, buffer_index: usize) -> anyhow::Result<()> {
		use graphics::debug;
		profiling::scope!("record:RenderVoxel");

		buffer.begin_label("Draw:Voxel", debug::LABEL_COLOR_DRAW);
		{
			self.drawable.bind_pipeline(buffer);

			let submitted_instances = self.instance_buffer.submitted();
			buffer.bind_vertex_buffers(0, vec![&self.model_cache.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&submitted_instances.buffer], vec![0]);
			buffer.bind_index_buffer(&self.model_cache.index_buffer, 0);

			for instances in submitted_instances.categories.iter() {
				let id = match instances.id {
					Some(id) => id,
					None => continue,
				};
				if instances.count() < 1 {
					continue;
				}
				let (model, index_start, vertex_offset) = match self.model_cache.get(&id) {
					Some(entry) => entry,
					None => continue,
				};
				let label = format!("Draw:Voxel({})", block::Lookup::lookup_id(id).unwrap());
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
						&model.descriptor_set(),
					],
				);

				// Draw based on the model
				buffer.draw(
					model.index_count(),
					*index_start,
					instances.count(),
					instances.start(),
					*vertex_offset,
				);

				buffer.end_label();
			}
		}
		buffer.end_label();

		Ok(())
	}
}
