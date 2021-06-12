use crystal_sphinx::{plugin, CrystalSphinx};
use engine::utility::VoidResult;
use temportal_engine as engine;
use temportal_engine_editor as editor;

pub fn run(_config: plugin::Config) -> VoidResult {
	#[cfg(feature = "profile")]
	{
		engine::profiling::optick::start_capture();
	}

	engine::logging::init::<CrystalSphinx>(Some("_editor"))?;
	let mut engine = engine::Engine::new()?;

	editor::Editor::initialize::<CrystalSphinx>()?;
	if editor::Editor::read().run_commandlets()? {
		return Ok(());
	}

	let mut window = engine::window::Window::builder()
		.with_title("Crystal Sphinx Editor")
		.with_size(1280.0, 720.0)
		.with_resizable(true)
		.with_application::<CrystalSphinx>()
		.with_clear_color([0.0, 0.0, 0.0, 1.0].into())
		.build(&engine)?;

	let chain = window.create_render_chain(engine::graphics::renderpass::Info::default())?;
	let ui = editor::ui::Ui::create(&window, &mut engine, &chain)?;

	let workspace = editor::ui::Workspace::new();
	ui.write().unwrap().add_element(&workspace);

	engine.run(chain.clone());

	#[cfg(feature = "profile")]
	{
		use engine::Application;
		engine::profiling::optick::stop_capture(CrystalSphinx::name());
	}
	Ok(())
}
