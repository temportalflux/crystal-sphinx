use crystal_sphinx::{plugin, CrystalSphinx};
use engine::{graphics::chain::procedure::DefaultProcedure, Application};

use anyhow::Result;
pub mod block;

pub fn register_asset_types(manager: &mut editor::asset::Manager) {
	manager.register::<crystal_sphinx::block::Block, block::BlockEditorMetadata>();
}

pub fn run(_config: plugin::Config) -> Result<()> {
	engine::logging::init(&engine::logging::default_path(
		CrystalSphinx::name(),
		Some("_editor"),
	))?;
	let mut engine = engine::Engine::new()?;
	crystal_sphinx::register_asset_types();

	editor::Editor::initialize::<CrystalSphinx>()?;
	crate::register_asset_types(editor::Editor::write().asset_manager_mut());
	if editor::Editor::read().run_commandlets()? {
		return Ok(());
	}

	engine::window::Window::builder()
		.with_title("Crystal Sphinx Editor")
		.with_size(1280.0, 720.0)
		.with_resizable(true)
		.with_application::<CrystalSphinx>()
		.build(&mut engine)?;

	let render_phase = {
		let arc = engine.display_chain().unwrap();
		let mut chain = arc.write().unwrap();
		chain.apply_procedure::<DefaultProcedure>()?.into_inner()
	};
	let ui = engine::ui::egui::Ui::create(&mut engine, &render_phase)?;

	let workspace = editor::ui::Workspace::new();
	ui.write().unwrap().add_element(&workspace);

	let engine = engine.into_arclock();
	engine::Engine::run(engine.clone(), || {})
}
