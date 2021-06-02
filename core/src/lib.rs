//! Crystal Sphinx is a game written in Rust using TemportalEngine which is heavily inspired by Minecraft.
//! Its a voxel/block based game that enthusiastically supports multiplayer and creativity.
//! It diverges from the Minecraft experience, however, in that it is not a currated game with a specific set of rules / design expectations.
//! CS' ethusiastic support of creavitity extends to both enabling players in the core game systems,
//! as well as enabling the community to easily slot in their own modules/plugins/mods to change their experience.
//! Crystal Sphinx (and TemportalEngine) are both entirely open source, and as such are easily modifiable by the
//! community to further support the aforementioned module development.
//!
//! TODO:
//! will need to separate to its own git repository at some point https://stosb.com/blog/retaining-history-when-moving-files-across-repositories-in-git/
//! this operation will need to retain move history (i.e. `git log --name-only --format=format: --follow -- path/to/file | sort -u`)
//!
//! Library Notes:
//! - [libloading](https://docs.rs/libloading/0.7.0/libloading/) for plugin loading/execution. [See guide for more.](https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html)
//! - [async asset loading](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html)
//! - [laminar](https://crates.io/crates/laminar) as a replacement for Game Networking Sockets
//! - [physx](https://crates.io/crates/physx)
//! - [profiling](https://crates.io/crates/profiling)
//! - [cryptography](https://crates.io/crates/rustls)
//! - [noise](https://crates.io/crates/noise) for randomization and noise in chunk generation
//! - [specs](https://crates.io/crates/specs) [book](https://specs.amethyst.rs/docs/tutorials)
//! - [anymap](https://crates.io/crates/anymap)
//!

use engine::{utility::VoidResult, Application};
pub use temportal_engine as engine;

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
	let mut engine = engine::Engine::new::<CrystalSphinx>()?;

	let mut window = engine::window::Window::builder()
		.with_title(CrystalSphinx::display_name())
		.with_size(1280.0, 720.0)
		.with_resizable(true)
		.with_application::<CrystalSphinx>()
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
		.with_tree_root(engine::ui::make_widget!(crate::engine::ui::content_box))
		.attach_system(
			&mut engine,
			&chain,
			Some(CrystalSphinx::get_asset_id("render_pass/ui_subpass").as_string()),
		)?;

	engine.run(chain.clone());
	Ok(())
}
