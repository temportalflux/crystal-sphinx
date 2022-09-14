use crate::{
	app::state, client::model::blender::render::RenderModel, common, entity,
	graphics::voxel::camera::Camera,
};
use engine::graphics;
use std::sync::{Arc, RwLock, Weak};

pub mod blender;
mod gather_entities_to_render;
pub use gather_entities_to_render::*;
pub mod mesh;

pub struct Model {
	mesh: mesh::Mesh,
}

#[derive(Clone)]
pub struct SystemDependencies {
	pub storage: Weak<RwLock<common::network::Storage>>,

	pub render_chain: Weak<RwLock<graphics::Chain>>,
	pub render_phase: Weak<graphics::procedure::Phase>,

	pub camera: Weak<RwLock<Camera>>,
	pub world: Weak<RwLock<entity::World>>,

	pub blender_model_cache: Arc<blender::model::Cache>,
}
struct RenderSystemObjects {
	render: Arc<RwLock<RenderModel>>,
	system: Arc<RwLock<GatherEntitiesToRender>>,
}
impl SystemDependencies {
	pub fn add_state_listener(self, app_state: &Arc<RwLock<state::Machine>>) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		// In theory, this struct will be kept around as long as the storage callback exists.
		// This is fine because we want the models to always exist
		// as long as the game is running (even if not present in the world),
		// and the rest of the data are weak references.
		let callback_deps = self;
		Storage::<RenderSystemObjects>::default()
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				use crate::common::network::mode;

				// This system should only be active/present while
				// in-game on the (integrated or dedicated) client.
				if !mode::get().contains(mode::Kind::Client) {
					return Ok(None);
				}

				let Self {
					storage,
					render_chain,
					render_phase,
					camera,
					world,
					blender_model_cache,
				} = callback_deps.clone();
				let chain = render_chain.upgrade().unwrap();
				let phase = render_phase.upgrade().unwrap();
				let camera = camera.upgrade().unwrap();

				let render = RenderModel::create(&chain, &phase, camera, blender_model_cache)?;
				let system = GatherEntitiesToRender::create(world.clone());

				return Ok(Some(RenderSystemObjects { render, system }));
			});
	}
}
