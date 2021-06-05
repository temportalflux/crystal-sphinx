use crystal_sphinx;
use crystal_sphinx::CrystalSphinx;
use engine::utility::VoidResult;
use temportal_engine as engine;
use temportal_engine_editor as editor;

fn main() -> VoidResult {
	#[cfg(feature = "profile")]
	{
		engine::profiling::optick::start_capture();
	}

	let mut engine = engine::Engine::new::<CrystalSphinx>(Some("_editor"))?;

	editor::Editor::initialize::<CrystalSphinx>()?;
	if editor::Editor::read().run_commandlets()? {
		return Ok(());
	}

	let mut window = engine::window::Window::builder()
		.with_title("Crystal Sphinx Editor")
		.with_size(1280.0, 720.0)
		.with_resizable(true)
		.with_application::<CrystalSphinx>()
		.with_clear_color([0.02, 0.02, 0.02, 1.0].into())
		.build(&engine)?;

	let chain = window.create_render_chain(engine::graphics::renderpass::Info::default())?;
	let ui = editor::ui::Ui::create(&window, &mut engine, &chain)?;

	let workspace = editor::ui::Workspace::new();
	ui.write().unwrap().add_element(&workspace);

	engine.run(chain.clone());

	#[cfg(feature = "profile")]
	{
		use engine::Application;
		engine::profiling::optick::stop_capture(crystal_sphinx::CrystalSphinx::name());
	}
	Ok(())
}
