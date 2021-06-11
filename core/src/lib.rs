//! Crystal Sphinx is a game written in Rust using TemportalEngine which is heavily inspired by Minecraft.
//! Its a voxel/block based game that enthusiastically supports multiplayer and creativity.
//! It diverges from the Minecraft experience, however, in that it is not a currated game with a specific set of rules / design expectations.
//! CS' ethusiastic support of creavitity extends to both enabling players in the core game systems,
//! as well as enabling the community to easily slot in their own modules/plugins/mods to change their experience.
//! Crystal Sphinx (and TemportalEngine) are both entirely open source, and as such are easily modifiable by the
//! community to further support the aforementioned module development.
//!
//! Library Notes:
//! - [libloading](https://docs.rs/libloading/0.7.0/libloading/) for plugin loading/execution. [See guide for more.](https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html)
//! - [async asset loading](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html)
//! - [networking - laminar](https://crates.io/crates/laminar) as a replacement for Game Networking Sockets
//! - [physics - rapier](https://crates.io/crates/rapier3d)
//! - [profiling](https://crates.io/crates/profiling)
//! - [cryptography](https://crates.io/crates/rustls)
//! - [noise](https://crates.io/crates/noise) for randomization and noise in chunk generation
//! - [specs](https://crates.io/crates/specs) [book](https://specs.amethyst.rs/docs/tutorials)
//! - [anymap](https://crates.io/crates/anymap)
//!

use engine::{utility::VoidResult, Application};
pub use temportal_engine as engine;

#[path = "ui/mod.rs"]
pub mod ui;

#[path = "plugin/mod.rs"]
pub mod plugin;

pub struct CrystalSphinx();
impl Application for CrystalSphinx {
	fn name() -> &'static str {
		std::env!("CARGO_PKG_NAME")
	}
	fn display_name() -> &'static str {
		"Crystal Sphinx"
	}
	fn location() -> &'static str {
		std::env!("CARGO_MANIFEST_DIR")
	}
	fn version() -> u32 {
		engine::utility::make_version(
			std::env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
			std::env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
			std::env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
		)
	}
}

pub fn run() -> VoidResult {
	engine::logging::init::<CrystalSphinx>(None)?;
	let mut engine = engine::Engine::new()?;
	engine.scan_paks()?;

	// TODO: Scan all plugins in a specific directory (always_loaded vs plugins for a specific save)
	// TODO: Scan any pak files which exist for each plugin
	let _ = plugin::Module::load(std::path::PathBuf::from("vanilla.dll").as_path())?;

	let mut window = engine::window::Window::builder()
		.with_title(CrystalSphinx::display_name())
		.with_size(1280.0, 720.0)
		.with_resizable(true)
		.with_application::<CrystalSphinx>()
		.with_clear_color([0.0, 0.05, 0.1, 1.0].into())
		.build(&engine)?;

	let chain = window.create_render_chain({
		engine::asset::Loader::load_sync(&CrystalSphinx::get_asset_id("render_pass/root"))?
			.downcast::<engine::graphics::render_pass::Pass>()
			.unwrap()
			.as_graphics()?
	})?;

	engine::ui::System::new(&chain)?
		.with_engine_shaders()?
		.with_all_fonts()?
		.with_tree_root(engine::ui::make_widget!(ui::root::root))
		.attach_system(
			&mut engine,
			&chain,
			Some(CrystalSphinx::get_asset_id("render_pass/ui_subpass").as_string()),
		)?;

	engine.run(chain.clone());
	Ok(())
}
