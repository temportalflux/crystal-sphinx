use crate::{
	app::state::{self, ArcLockMachine},
	graphics::voxel::{camera, model, Instance, InstanceFlags},
	CrystalSphinx,
};
use engine::{
	asset,
	graphics::{
		self, buffer, command, flags, structs, utility::NamedObject, ArcRenderChain, Drawable,
		RenderChain, RenderChainElement, Uniform,
	},
	math::nalgebra::{Translation3, Vector2, Vector3},
	task::{self, ScheduledTask},
	utility::AnyError,
	Application,
};
use enumset::EnumSet;
use std::sync::{Arc, RwLock};

static ID: &'static str = "render-voxel";

pub type ArcLockRenderVoxel = Arc<RwLock<RenderVoxel>>;
pub struct RenderVoxel {
	pending_gpu_signals: Vec<Arc<command::Semaphore>>,
	drawable: Drawable,
	vertex_buffer: Arc<buffer::Buffer>,
	index_buffer: Arc<buffer::Buffer>,
	instance_buffer: Arc<buffer::Buffer>,
	camera_uniform: Uniform,
	camera: Arc<RwLock<camera::Camera>>,
	model_cache: model::ArcLockCache,
}

impl RenderVoxel {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/world_subpass")
	}

	pub fn add_state_listener(
		app_state: &ArcLockMachine,
		render_chain: &ArcRenderChain,
		model_cache: &model::ArcLockCache,
		camera: &Arc<RwLock<camera::Camera>>,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_render_chain = render_chain.clone();
		let callback_model_cache = model_cache.clone();
		let callback_camera = Arc::downgrade(&camera);
		Storage::<ArcLockRenderVoxel>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				let mut render_chain = callback_render_chain.write().unwrap();
				let arc_camera = callback_camera.upgrade().unwrap();
				match Self::create(&mut render_chain, &callback_model_cache, arc_camera) {
					Ok(arclocked) => Some(arclocked),
					Err(err) => {
						log::error!(target: ID, "{}", err);
						return None;
					}
				}
			});
	}

	fn create(
		render_chain: &mut RenderChain,
		model_cache: &model::ArcLockCache,
		camera: Arc<RwLock<camera::Camera>>,
	) -> Result<ArcLockRenderVoxel, AnyError> {
		let subpass_id = Self::subpass_id();
		let render_chunks = Self::new(&render_chain, model_cache.clone(), camera)?.arclocked();
		render_chain.add_render_chain_element(Some(subpass_id.as_string()), &render_chunks)?;
		Ok(render_chunks)
	}

	fn new(
		render_chain: &RenderChain,
		model_cache: model::ArcLockCache,
		camera: Arc<RwLock<camera::Camera>>,
	) -> Result<Self, AnyError> {
		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/vertex"))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/world/fragment"))?;

		let max_instances = 1; // TODO: what to do with this?
		let (vbuffer_size, ibuffer_size) = {
			let model_cache = model_cache.read().unwrap();
			(
				model_cache.vertex_buffer_size(),
				model_cache.index_buffer_size(),
			)
		};

		let vertex_buffer = buffer::Buffer::create_gpu(
			Some(format!("RenderVoxel.VertexBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::VERTEX_BUFFER,
			vbuffer_size,
			None,
		)?;

		let instance_buffer = buffer::Buffer::create_gpu(
			Some(format!("RenderVoxel.InstanceBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::VERTEX_BUFFER,
			std::mem::size_of::<Instance>() * max_instances,
			None,
		)?;

		let index_buffer = buffer::Buffer::create_gpu(
			Some(format!("RenderVoxel.IndexBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::INDEX_BUFFER,
			ibuffer_size,
			Some(flags::IndexType::UINT32),
		)?;

		let camera_uniform =
			Uniform::new::<camera::UniformData, &str>("RenderVoxel.Camera", &render_chain)?;

		Ok(Self {
			pending_gpu_signals: Vec::new(),
			drawable,
			vertex_buffer,
			instance_buffer,
			index_buffer,
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
	) -> Result<Vec<Arc<command::Semaphore>>, AnyError> {
		let gpu_signals = Vec::new();

		self.drawable.create_shaders(render_chain)?;
		self.camera_uniform.write_descriptor_sets(render_chain);

		let model_cache = self.model_cache.read().unwrap();
		let (vertices, indices) = model_cache.buffer_data();

		let instances: Vec<Instance> = vec![Instance {
			chunk_coordinate: Vector3::default().into(),
			model_matrix: Translation3::new(0.0, 0.0, 0.0).to_homogeneous().into(),
			instance_flags: InstanceFlags {
				faces: EnumSet::all(),
			}
			.into(),
		}];

		graphics::TaskGpuCopy::new(
			self.vertex_buffer.wrap_name(|v| format!("Write({})", v)),
			&render_chain,
		)?
		.begin()?
		.stage(&vertices[..])?
		.copy_stage_to_buffer(&self.vertex_buffer)
		.end()?
		.add_signal_to(&mut self.pending_gpu_signals)
		.send_to(task::sender());

		graphics::TaskGpuCopy::new(
			self.index_buffer.wrap_name(|v| format!("Write({})", v)),
			&render_chain,
		)?
		.begin()?
		.stage(&indices[..])?
		.copy_stage_to_buffer(&self.index_buffer)
		.end()?
		.add_signal_to(&mut self.pending_gpu_signals)
		.send_to(task::sender());

		graphics::TaskGpuCopy::new(
			self.instance_buffer.wrap_name(|v| format!("Write({})", v)),
			&render_chain,
		)?
		.begin()?
		.stage(&instances[..])?
		.copy_stage_to_buffer(&self.instance_buffer)
		.end()?
		.add_signal_to(&mut self.pending_gpu_signals)
		.send_to(task::sender());

		Ok(gpu_signals)
	}

	fn on_render_chain_constructed(
		&mut self,
		render_chain: &RenderChain,
		resolution: &Vector2<f32>,
		subpass_id: &Option<String>,
	) -> Result<(), AnyError> {
		use graphics::pipeline::{state::*, Pipeline};
		Ok(self.drawable.create_pipeline(
			render_chain,
			vec![self.camera_uniform.layout()],
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
				),
			subpass_id,
		)?)
	}

	fn destroy_render_chain(&mut self, render_chain: &RenderChain) -> Result<(), AnyError> {
		self.drawable.destroy_pipeline(render_chain)?;
		Ok(())
	}

	fn prerecord_update(
		&mut self,
		_render_chain: &graphics::RenderChain,
		_buffer: &command::Buffer,
		frame: usize,
		resolution: &Vector2<f32>,
	) -> Result<bool, AnyError> {
		let data = self.camera.read().unwrap().as_uniform_data(resolution);
		self.camera_uniform.write_data(frame, &data)?;
		Ok(false)
	}

	fn record_to_buffer(&self, buffer: &mut command::Buffer, frame: usize) -> Result<(), AnyError> {
		use graphics::debug;

		buffer.begin_label("Draw:Debug", debug::LABEL_COLOR_DRAW);
		{
			self.drawable.bind_pipeline(buffer);
			self.drawable
				.bind_descriptors(buffer, vec![&self.camera_uniform.get_set(frame).unwrap()]);

			buffer.bind_vertex_buffers(0, vec![&self.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&self.instance_buffer], vec![0]);
			buffer.bind_index_buffer(&self.index_buffer, 0);

			buffer.draw(36, 0, 1, 0, 0);
		}
		buffer.end_label();

		Ok(())
	}

	fn take_gpu_signals(&mut self) -> Vec<Arc<command::Semaphore>> {
		self.pending_gpu_signals.drain(..).collect()
	}
}
