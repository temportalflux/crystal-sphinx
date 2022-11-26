use crate::{
	app::state, client::model::blender::render::RenderModel, common, entity,
	graphics::voxel::camera::Camera,
};
use engine::graphics;
use std::sync::{Arc, Mutex, RwLock, Weak};

pub mod blender;
mod gather_entities_to_render;
pub use gather_entities_to_render::*;
pub mod instance;
mod player_model;
pub use player_model::*;
pub mod texture;

#[derive(Clone)]
pub struct SystemDependencies {
	pub storage: Weak<RwLock<common::network::Storage>>,

	pub render_chain: Weak<RwLock<graphics::Chain>>,
	pub render_phase: Weak<graphics::procedure::Phase>,

	pub camera: Weak<RwLock<Camera>>,
	pub world: Weak<RwLock<entity::World>>,

	pub blender_model_cache: Arc<blender::model::Cache>,
	pub texture_cache: Arc<Mutex<texture::Cache>>,
}
struct RenderSystemObjects {
	#[allow(dead_code)]
	render: Arc<RwLock<RenderModel>>,
	#[allow(dead_code)]
	system: Arc<RwLock<GatherEntitiesToRender>>,
}
impl SystemDependencies {
	pub fn add_state_listener(self, app_state: &Arc<RwLock<state::Machine>>) {
		use state::{
			storage::{Callback, Storage},
			OperationKey,
			State::*,
			Transition::*,
		};

		// In theory, this struct will be kept around as long as the storage callback exists.
		// This is fine because we want the models to always exist
		// as long as the game is running (even if not present in the world),
		// and the rest of the data are weak references.
		let callback_deps = self;
		Storage::<RenderSystemObjects>::default()
			.create_when(OperationKey(None, Some(Enter), Some(InGame)))
			.destroy_when(OperationKey(Some(InGame), Some(Exit), None))
			.with_callback(Callback::recurring(move || {
				use crate::common::network::mode;

				// This system should only be active/present while
				// in-game on the (integrated or dedicated) client.
				if !mode::get().contains(mode::Kind::Client) {
					return Ok(None);
				}

				let Self {
					storage: _,
					render_chain,
					render_phase,
					camera,
					world,
					blender_model_cache,
					texture_cache,
				} = callback_deps.clone();
				let chain = render_chain.upgrade().unwrap();
				let phase = render_phase.upgrade().unwrap();
				let camera = camera.upgrade().unwrap();

				let instance_buffer = Arc::new(RwLock::new(instance::Buffer::new(
					&chain.read().unwrap().allocator()?,
					std::mem::size_of::<instance::Instance>() * 30, // magic number, entity count will be way higher than 30
				)?));

				let render = RenderModel::create(
					&chain,
					&phase,
					camera,
					blender_model_cache,
					instance_buffer.clone(),
					texture_cache.clone(),
				)?;
				let system =
					GatherEntitiesToRender::create(world.clone(), &instance_buffer, &texture_cache);

				return Ok(Some(RenderSystemObjects { render, system }));
			}))
			.build(&app_state);
	}
}
