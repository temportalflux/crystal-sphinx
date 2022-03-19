use std::{
	path::PathBuf,
	sync::{Arc, RwLock},
};

use crystal_sphinx::CrystalSphinx;
use editor::{asset, ui::Workspace, Editor};
use engine::{
	graphics::{chain::procedure::DefaultProcedure, Chain},
	task::PinFutureResultLifetime,
	window::Window,
	Application, Engine, EventLoop,
};

pub mod blender_model;
pub mod block;

pub struct Runtime {
	window: Option<Window>,
	workspace: Option<Arc<RwLock<Workspace>>>,
}
impl Runtime {
	pub fn new() -> Self {
		Self {
			window: None,
			workspace: None,
		}
	}
}
impl engine::Runtime for Runtime {
	fn logging_path() -> PathBuf {
		engine::logging::default_path(CrystalSphinx::name(), Some("_editor"))
	}

	fn register_asset_types() {
		crystal_sphinx::Runtime::register_asset_types();
	}

	fn initialize<'a>(&'a self, _engine: Arc<RwLock<Engine>>) -> PinFutureResultLifetime<'a, bool> {
		Box::pin(async move {
			self.create_editor().await?;
			let ran_commandlets = editor::Editor::run_commandlets().await;
			Ok(!ran_commandlets)
		})
	}

	fn create_display(
		&mut self,
		engine: &Arc<RwLock<Engine>>,
		event_loop: &EventLoop<()>,
	) -> anyhow::Result<()> {
		let window = engine::window::Window::builder()
			.with_title("Crystal Sphinx Editor")
			.with_size(1280.0, 720.0)
			.with_resizable(true)
			.with_application::<CrystalSphinx>()
			.build(event_loop)?;

		let render_phase = {
			let arc = window.graphics_chain();
			let mut chain = arc.write().unwrap();
			chain.apply_procedure::<DefaultProcedure>()?.into_inner()
		};

		let ui = engine::ui::egui::Ui::create(&window, &render_phase)?;
		if let Ok(mut engine) = engine.write() {
			engine.add_winit_listener(&ui);
		}

		self.window = Some(window);

		let workspace = Workspace::new();
		ui.write().unwrap().add_element(&workspace);
		self.workspace = Some(workspace);

		Ok(())
	}

	fn get_display_chain(&self) -> Option<&Arc<RwLock<Chain>>> {
		self.window.as_ref().map(|window| window.graphics_chain())
	}
}

impl Runtime {
	async fn create_editor(&self) -> anyhow::Result<()> {
		let editor = Editor::new(self.create_asset_manager()).await?;
		Editor::initialize(editor)
	}

	fn create_asset_manager(&self) -> asset::Manager {
		use crate::{blender_model::BlenderModelEditorOps, block::BlockEditorOps};
		let mut manager = asset::Manager::new();
		editor::register_asset_types(&mut manager);
		manager.register::<BlockEditorOps>();
		manager.register::<BlenderModelEditorOps>();
		manager
	}
}
