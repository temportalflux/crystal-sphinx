use crate::{
	app::state::{self, ArcLockMachine},
	block,
	client::{model::blender::model, world::chunk},
	common::network::Storage,
	graphics::voxel::camera,
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

static ID: &'static str = "render-entity";

/// Management of non-block models and executing draw-calls for entities during frame render.
/// Exists only as long as the user is in a world
/// (it is saved to session storage, created when entering a game and destroyed upon leaving).
pub struct RenderModel {
	drawable: Drawable,
	camera_uniform: Uniform,
	camera: Arc<RwLock<camera::Camera>>,
	model_cache: Arc<model::Cache>,
}

impl RenderModel {
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
		let callback_camera = camera;
		// This arc will be kept around as long as the storage callback exists,
		// which is fine because we want the models to always exist
		// as long as the game is running (even if not present in the world).
		let callback_model_cache = model_cache;
		Storage::<Arc<RwLock<Self>>>::default()
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
				let model_cache = callback_model_cache.clone();

				Ok(
					match Self::create(&chain, &phase, arc_camera, model_cache) {
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
	) -> Result<Arc<RwLock<Self>>> {
		log::info!(target: ID, "Initializing");
		let instance = Self::new(&chain.read().unwrap(), camera, model_cache)?.arclocked();

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
	) -> Result<Self> {
		log::trace!(target: ID, "Creating renderer");

		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		//drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/entity/vertex"))?;
		//drawable.add_shader(&CrystalSphinx::get_asset_id("shaders/entity/fragment"))?;

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

		/*
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
		*/
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
		Ok(RequiresRecording::NotRequired)
	}

	fn record(&mut self, buffer: &mut command::Buffer, buffer_index: usize) -> anyhow::Result<()> {
		use graphics::debug;
		profiling::scope!("record:RenderModel");

		Ok(())
	}
}