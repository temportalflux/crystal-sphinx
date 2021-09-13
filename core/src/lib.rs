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
//! - [gltf](https://crates.io/crates/gltf) for loading scenes/rigs/animations (replacement for assimp)
//! - [asset_manager](https://github.com/a1phyr/assets_manager) for inspiration on hot-reloading assets
//! - hecs to potentially replace specs
//! - wgpu to replace vulkan-rs as the rendering backend
//!
//! Rust's support for dyynamically-loaded plugins (*.dll, etc) is not great yet. As such, plugins cannot be loaded at runtime without increasing the complexity for plugin creators by orders of magnitude. Therefore, the game and editor must be compiled with all desired plugins/crates ahead of time. This offloads some overhead to plugin-pack creators, but can be supplemented by better tooling on that end of the toolchain.
//! Links for reference on DLLs:
//! - https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html
//! - https://github.com/rust-lang/log/issues/66
//! - https://github.com/rust-lang/log/issues/421
//!
//! https://grafana.com/ could be neat for monitoring server usage
//!

use engine::{utility::VoidResult, Application};
pub use temportal_engine as engine;

pub mod account;
pub mod network;
pub mod plugin;
pub mod ui;

mod server;
pub use server::*;

pub struct CrystalSphinx();
impl Application for CrystalSphinx {
	fn name() -> &'static str {
		std::env!("CARGO_PKG_NAME")
	}
	fn version() -> semver::Version {
		semver::Version::parse(std::env!("CARGO_PKG_VERSION")).unwrap()
	}
}

pub fn run(config: plugin::Config) -> VoidResult {
	let user_id = std::env::args()
		.find_map(|arg| arg.strip_prefix("-user=").map(|s| s.to_owned()))
		.unwrap();
	let logid = std::env::args()
		.find_map(|arg| arg.strip_prefix("-logid=").map(|s| s.to_owned()))
		.unwrap();
	let log_path = {
		let mut log_path = std::env::current_dir().unwrap().to_path_buf();
		log_path.push(format!("{}_{}.log", CrystalSphinx::name(), logid));
		log_path
	};
	engine::logging::init(&log_path)?;

	#[cfg(feature = "profile")]
	{
		log::info!(target: "profile", "Starting profiling capture");
		engine::profiling::optick::start_capture();
	}

	// Load bundled plugins so they can be used throughout the instance
	if let Ok(mut manager) = plugin::Manager::write() {
		manager.load(config);
	}

	let mut engine = engine::Engine::new()?;
	engine.scan_paks()?;

	if let Ok(mut guard) = account::ClientRegistry::write() {
		(*guard).scan_accounts()?;
		let _account = (*guard).login_as(&user_id);
	}

	engine::network::Builder::default()
		.with_port(25565)
		.with_args()
		.with_registrations_in(network::packet::register_types)
		.spawn()?;

	if engine::network::Network::local_data().is_server() {
		let mut server_save_path = std::env::current_dir().unwrap();
		server_save_path.push("saves");
		server_save_path.push("tmp");
		if let Ok(mut guard) = Server::write() {
			(*guard) = Some(Server::load(&server_save_path)?);
		}
	}

	if engine::network::Network::local_data().is_client() {
		engine::window::Window::builder()
			.with_title("Crystal Sphinx")
			.with_size(1280.0, 720.0)
			.with_resizable(true)
			.with_application::<CrystalSphinx>()
			.with_clear_color([0.0, 0.05, 0.1, 1.0].into())
			.build(&mut engine)?;
		engine.render_chain_write().unwrap().set_render_pass_info(
			engine::asset::Loader::load_sync(&CrystalSphinx::get_asset_id("render_pass/root"))?
				.downcast::<engine::graphics::render_pass::Pass>()
				.unwrap()
				.as_graphics()?,
		);

		/*
		engine::ui::System::new(engine.render_chain().unwrap())?
			.with_engine_shaders()?
			.with_all_fonts()?
			.with_tree_root(engine::ui::make_widget!(ui::root::root))
			.attach_system(
				&mut engine,
				Some(CrystalSphinx::get_asset_id("render_pass/ui_subpass").as_string()),
			)?;
		*/

		/*
		let mut main_menu_music = engine::asset::WeightedIdList::default();
		if let Ok(manager) = plugin::Manager::read() {
			manager.register_main_menu_music(&mut main_menu_music);
		}

		{
			use engine::audio::source::Source;
			main_menu_music
				.iter()
				.map(|(_, id)| id.clone())
				.collect::<engine::audio::source::Sequence>()
				.and_play(None)
				.register_to(&mut engine);
		}
		*/

		/*
		let _source = {
			use rand::Rng;
			let mut rng = rand::thread_rng();
			match main_menu_music.pick(rng.gen_range(0..main_menu_music.total_weight())) {
				Some(id) => {
					let mut audio_system = engine::audio::System::write()?;
					match audio_system.create_sound(id) {
						Ok(mut source) => {
							source.play(None);
							Some(source)
						}
						Err(e) => {
							log::error!("Failed to load sound {}: {}", id, e);
							None
						}
					}
				}
				None => {
					log::warn!("Failed to find any main menu music");
					None
				}
			}
		};
		*/

		network::packet::Handshake::connect_to_server()?;
	}

	let engine = engine.into_arclock();
	engine::Engine::run(engine.clone(), || {
		if let Ok(mut guard) = account::ClientRegistry::write() {
			(*guard).logout();
		}
		#[cfg(feature = "profile")]
		{
			log::info!(target: "profile", "Stopping profiling capture");
			engine::profiling::optick::stop_capture(CrystalSphinx::name());
		}
	})
}
