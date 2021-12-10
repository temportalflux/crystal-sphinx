use crystal_sphinx::{plugin, CrystalSphinx};
use engine::{utility::VoidResult, Application};

pub mod block;

pub fn register_asset_types(manager: &mut editor::asset::Manager) {
	manager.register::<crystal_sphinx::block::Block, block::BlockEditorMetadata>();
}

pub fn run(_config: plugin::Config) -> VoidResult {
	#[cfg(feature = "profile")]
	{
		engine::profiling::optick::start_capture();
	}

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
		.with_clear_color([0.0, 0.0, 0.0, 1.0].into())
		.build(&mut engine)?;

	let ui = engine::ui::egui::Ui::create(&mut engine)?;

	let workspace = editor::ui::Workspace::new();
	ui.write().unwrap().add_element(&workspace);

	let engine = engine.into_arclock();
	engine::Engine::run(engine.clone(), || {
		#[cfg(feature = "profile")]
		{
			engine::profiling::optick::stop_capture(CrystalSphinx::name());
		}
	})
}
