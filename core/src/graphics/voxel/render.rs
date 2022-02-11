use crate::{
	app::state::{self, ArcLockMachine},
	block,
	client::world::chunk::cache,
	common::network::Storage,
	graphics::voxel::{
		camera,
		instance::{self, Instance},
		model,
	},
	CrystalSphinx,
};
use engine::{
	asset,
	graphics::{
		self, command, flags, structs, ArcRenderChain, Drawable, RenderChain, RenderChainElement,
		Uniform,
	},
	math::nalgebra::Vector2,
	utility::Result,
	Application,
};
use std::sync::{Arc, RwLock, Weak};

static ID: &'static str = "render-voxel";

pub type ArcLockRenderVoxel = Arc<RwLock<RenderVoxel>>;
pub struct RenderVoxel {
	pending_gpu_signals: Vec<Arc<command::Semaphore>>,
	drawable: Drawable,
	instance_buffer: instance::Buffer,
	camera_uniform: Uniform,
	camera: Arc<RwLock<camera::Camera>>,
	model_cache: Arc<model::Cache>,
}

impl RenderVoxel {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/world_subpass")
	}

	pub fn add_state_listener(
		app_state: &ArcLockMachine,
		storage: Weak<RwLock<Storage>>,
		render_chain: &ArcRenderChain,
		camera: &Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		mut gpu_signals: Vec<Arc<command::Semaphore>>,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_render_chain = Arc::downgrade(&render_chain);
		let callback_storage = storage.clone();
		let callback_model_cache = model_cache;
		let callback_camera = Arc::downgrade(&camera);
		let pending_gpu_signals = gpu_signals.drain(..).collect::<Vec<_>>();
		Storage::<ArcLockRenderVoxel>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				profiling::scope!("init-render", ID);
				log::trace!(target: ID, "Received Enter(InGame) transition");
				let render_chain = callback_render_chain.upgrade().unwrap();
				let arc_camera = callback_camera.upgrade().unwrap();

				let chunk_cache = match callback_storage.upgrade() {
					Some(arc_storage) => {
						let storage = arc_storage.read().unwrap();
						match storage.client() {
							Some(arc_client) => {
								let client = arc_client.read().unwrap();
								client.chunk_cache().clone()
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
						render_chain,
						arc_camera,
						&callback_model_cache,
						chunk_cache,
						pending_gpu_signals.clone(),
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
		render_chain: Arc<RwLock<RenderChain>>,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: &Arc<model::Cache>,
		chunk_cache: cache::ArcLock,
		gpu_signals: Vec<Arc<command::Semaphore>>,
	) -> Result<ArcLockRenderVoxel> {
		log::info!(target: ID, "Initializing");
		let render_chunks = {
			let render_chain = render_chain.read().unwrap();
			Self::new(
				&render_chain,
				camera,
				model_cache.clone(),
				chunk_cache,
				gpu_signals,
			)?
			.arclocked()
		};

		let subpass_id = Self::subpass_id();
		let element = render_chunks.clone();
		engine::task::spawn(ID.to_owned(), async move {
			log::trace!(target: ID, "Adding to render chain");
			let mut render_chain = render_chain.write().unwrap();
			render_chain.add_render_chain_element(Some(subpass_id.as_string()), &element)?;
			Ok(())
		});

		Ok(render_chunks)
	}

	fn new(
		render_chain: &RenderChain,
		camera: Arc<RwLock<camera::Camera>>,
		model_cache: Arc<model::Cache>,
		chunk_cache: cache::ArcLock,
		pending_gpu_signals: Vec<Arc<command::Semaphore>>,
	) -> Result<Self> {
		log::trace!(target: ID, "Creating renderer");

		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/vertex"))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/fragment"))?;

		let instance_buffer = instance::Buffer::new(
			&render_chain,
			Arc::downgrade(&model_cache),
			Arc::downgrade(&chunk_cache),
		)?;

		let camera_uniform =
			Uniform::new::<camera::UniformData, &str>("RenderVoxel.Camera", &render_chain)?;

		Ok(Self {
			pending_gpu_signals,
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

impl RenderChainElement for RenderVoxel {
	fn name(&self) -> &'static str {
		ID
	}

	fn initialize_with(
		&mut self,
		render_chain: &mut RenderChain,
	) -> Result<Vec<Arc<command::Semaphore>>> {
		let gpu_signals = Vec::new();

		self.drawable.create_shaders(render_chain)?;
		self.camera_uniform.write_descriptor_sets(render_chain);

		Ok(gpu_signals)
	}

	fn on_render_chain_constructed(
		&mut self,
		render_chain: &RenderChain,
		resolution: &Vector2<f32>,
		subpass_id: &Option<String>,
	) -> Result<()> {
		use graphics::pipeline::{state::*, Pipeline};
		Ok(self.drawable.create_pipeline(
			render_chain,
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
				.set_viewport_state(Viewport::from(structs::Extent2D {
					width: resolution.x as u32,
					height: resolution.y as u32,
				}))
				.set_color_blending(
					color_blend::ColorBlend::default()
						.add_attachment(color_blend::Attachment::default()),
				)
				.with_depth_stencil(
					DepthStencil::default()
						.with_depth_test()
						.with_depth_write()
						.with_depth_bounds(0.0, 1.0)
						.with_depth_compare_op(flags::CompareOp::LESS),
				),
			subpass_id,
		)?)
	}

	fn destroy_render_chain(&mut self, render_chain: &RenderChain) -> Result<()> {
		self.drawable.destroy_pipeline(render_chain)?;
		Ok(())
	}

	fn prerecord_update(
		&mut self,
		render_chain: &graphics::RenderChain,
		_buffer: &command::Buffer,
		frame: usize,
		resolution: &Vector2<f32>,
	) -> Result<bool> {
		let data = self.camera.read().unwrap().as_uniform_data(resolution);
		self.camera_uniform.write_data(frame, &data)?;

		let (was_able_to_lock, mut instance_signals) =
			self.instance_buffer.prerecord_update(&render_chain)?;
		let should_record = !was_able_to_lock || !instance_signals.is_empty();
		self.pending_gpu_signals.append(&mut instance_signals);

		// If the instances change, we need to re-record the render
		Ok(should_record)
	}

	fn record_to_buffer(&self, buffer: &mut command::Buffer, frame: usize) -> Result<()> {
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
						&self.camera_uniform.get_set(frame).unwrap(),
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

	fn take_gpu_signals(&mut self) -> Vec<Arc<command::Semaphore>> {
		self.pending_gpu_signals.drain(..).collect()
	}
}
