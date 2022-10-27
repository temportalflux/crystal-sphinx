use crystal_sphinx::CrystalSphinx;
use editor::{asset, ui::Workspace, Editor};
use engine::{
	graphics::{chain::procedure::DefaultProcedure, Chain},
	math::nalgebra::Vector2,
	task::PinFutureResultLifetime,
	window::Window,
	Application, Engine, EventLoop,
};
use std::{
	path::PathBuf,
	sync::{Arc, RwLock},
};

pub mod blender_model;
pub mod block;

pub struct Runtime {
	window: Option<Window>,
	workspace: Option<Arc<RwLock<Workspace>>>,
	egui_context: Option<egui::Context>,
}
impl Runtime {
	pub fn new() -> Self {
		Self {
			window: None,
			workspace: None,
			egui_context: None,
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
		let mut egui_memory = editor::Editor::write().persistent_data.egui.take();
		log::debug!("{:?}", egui_memory.is_some());
		let (pos, size) = {
			let (pos, size) = match &mut egui_memory {
				Some(memory) => {
					let pos = memory
						.data
						.get_persisted::<[f64; 2]>(egui::Id::new("window::position"));
					let size = memory
						.data
						.get_persisted::<[f64; 2]>(egui::Id::new("window::resolution"));
					(pos, size)
				}
				None => (None, None),
			};
			log::debug!("{pos:?} {size:?}");
			let pos: Vector2<f64> = pos.unwrap_or([0.0, 0.0]).into();
			let size: Vector2<f64> = size.unwrap_or([1280.0, 720.0]).into();
			(pos, size)
		};

		let window = engine::window::Window::builder()
			.with_title("Crystal Sphinx Editor")
			.with_position(pos.x, pos.y)
			.with_size(size.x, size.y)
			.with_resizable(true)
			.with_application::<CrystalSphinx>()
			.build(event_loop)?;

		let render_phase = {
			let arc = window.graphics_chain();
			let mut chain = arc.write().unwrap();
			chain.apply_procedure::<DefaultProcedure>()?.into_inner()
		};

		let ui = engine::ui::egui::Ui::create(&window, &*event_loop, &render_phase)?;
		editor::ui::icons::Icon::load_all(ui.clone());
		if let Ok(mut engine) = engine.write() {
			engine.add_winit_listener(&ui);
		}

		// Apply the egui-memory and save off the context arc-handle.
		// This clones the handle, resulting in two contexts which refer to the same data.
		let context = ui.read().unwrap().context().clone();
		if let Some(memory) = egui_memory {
			// Can be written to a clone of context because context is an interior-mutable arc.
			// So the memory of a clone of context has the same memory as the original context.
			*context.memory() = memory;
		}
		// Save the context arc-handle.
		self.egui_context = Some(context);

		self.window = Some(window);

		let workspace = Workspace::new();
		ui.write().unwrap().add_element(&workspace);
		self.workspace = Some(workspace);

		Ok(())
	}

	fn get_display_chain(&self) -> Option<&Arc<RwLock<Chain>>> {
		self.window.as_ref().map(|window| window.graphics_chain())
	}

	fn on_event_loop_complete(&self) {
		let mut memory = self.egui_context.as_ref().map(|context| context.memory().clone());
		if let Some(memory) = &mut memory {
			if let Some(window) = &self.window {
				log::debug!("{:?}", window.position());
				memory
					.data
					.insert_persisted::<[f64; 2]>(egui::Id::new("window::position"), window.position());
			}
		}

		{
			let mut editor = editor::Editor::write();
			editor.persistent_data.egui = memory;
			if let Err(err) = editor.persistent_data.write_to_disk() {
				log::error!("{err:?}");
			}
		}
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
