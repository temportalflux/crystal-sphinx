use crate::{
	app::state::{self, ArcLockMachine},
	CrystalSphinx,
};
use engine::{
	asset,
	graphics::{command, ArcRenderChain, RenderChain, RenderChainElement},
	math::nalgebra::Vector2,
	utility::AnyError,
	Application,
};
use std::sync::{Arc, RwLock};

static ID: &'static str = "render-chunks";

pub type ArcLockRenderChunks = Arc<RwLock<RenderChunks>>;
pub struct RenderChunks {}

impl RenderChunks {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/world_subpass")
	}

	pub fn add_state_listener(app_state: &ArcLockMachine, render_chain: &ArcRenderChain) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_render_chain = render_chain.clone();
		Storage::<ArcLockRenderChunks>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				let mut render_chain = callback_render_chain.write().unwrap();
				Self::create(&mut render_chain)
			});
	}

	fn create(render_chain: &mut RenderChain) -> ArcLockRenderChunks {
		let subpass_id = Self::subpass_id();
		let render_chunks = Self::new().arclocked();
		let _ = render_chain.add_render_chain_element(Some(subpass_id.as_string()), &render_chunks);
		render_chunks
	}

	fn new() -> Self {
		Self {}
	}

	fn arclocked(self) -> ArcLockRenderChunks {
		Arc::new(RwLock::new(self))
	}
}

impl Drop for RenderChunks {
	fn drop(&mut self) {
		log::info!(target: ID, "Dropping from subpass({}).", Self::subpass_id());
	}
}

impl RenderChainElement for RenderChunks {
	fn name(&self) -> &'static str {
		ID
	}

	fn on_render_chain_constructed(
		&mut self,
		_render_chain: &RenderChain,
		_resolution: &Vector2<f32>,
		_subpass_id: &Option<String>,
	) -> Result<(), AnyError> {
		Ok(())
	}

	fn destroy_render_chain(&mut self, _render_chain: &RenderChain) -> Result<(), AnyError> {
		Ok(())
	}

	fn record_to_buffer(&self, _buffer: &mut command::Buffer, _frame: usize) -> Result<(), AnyError> {
		Ok(())
	}
}
