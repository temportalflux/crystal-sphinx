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
//! - `<https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html>`
//! - `<https://github.com/rust-lang/log/issues/66>`
//! - `<https://github.com/rust-lang/log/issues/421>`
//!
//! `<https://grafana.com/>` could be neat for monitoring server usage
//!

use engine::{utility::VoidResult, Application};

pub mod account;
pub mod app;
pub mod block;
pub mod commands;
pub mod debug;
pub mod entity;
pub mod graphics;
pub mod input;
pub mod network;
pub mod plugin;
pub mod ui;
pub mod world;

use std::sync::{Arc, RwLock};

pub struct CrystalSphinx();
impl Application for CrystalSphinx {
	fn name() -> &'static str {
		std::env!("CARGO_PKG_NAME")
	}
	fn version() -> semver::Version {
		semver::Version::parse(std::env!("CARGO_PKG_VERSION")).unwrap()
	}
}

pub fn register_asset_types() {
	let mut type_reg = engine::asset::TypeRegistry::get().write().unwrap();
	type_reg.register::<block::Block>();
}

pub fn run(config: plugin::Config) -> VoidResult {
	let logid = std::env::args()
		.find_map(|arg| arg.strip_prefix("-logid=").map(|s| s.to_owned()))
		.unwrap();
	let log_path = {
		let mut log_path = std::env::current_dir().unwrap().to_path_buf();
		log_path.push(format!("{}_{}.log", CrystalSphinx::name(), logid));
		log_path
	};
	engine::logging::init(&log_path)?;

	// Load bundled plugins so they can be used throughout the instance
	if let Ok(mut manager) = plugin::Manager::write() {
		manager.load(config);
	}

	let mut engine = engine::Engine::new()?;
	crate::register_asset_types();
	engine.scan_paks()?;

	entity::component::register_types();

	let input_user: Option<input::ArcLockUser>;
	let is_client = std::env::args().any(|arg| arg == "-client");
	let is_server = std::env::args().any(|arg| arg == "-server");
	assert_ne!(is_client, is_server);

	let app_state = app::state::Machine::new(app::state::State::Launching).arclocked();
	let entity_world = entity::ArcLockEntityWorld::default();
	let network_storage = network::storage::Storage::new(&app_state);
	network::task::Unload::add_state_listener(&app_state);

	if is_server {
		network::task::Load::load_dedicated_server(&app_state, &network_storage, &entity_world);
	} else {
		input_user = Some(input::init());
		network::task::Load::add_state_listener(&app_state, &network_storage, &entity_world);

		let account_id = if let Ok(mut client_reg) = account::ClientRegistry::write() {
			client_reg.scan_accounts()?;
			let user_name = std::env::args()
				.find_map(|arg| arg.strip_prefix("-user=").map(|s| s.to_owned()))
				.unwrap();
			let user_id = client_reg.ensure_account(&user_name)?;
			client_reg.login_as(&user_id)?;
			user_id
		} else {
			unimplemented!()
		};
		engine.add_system(
			entity::system::PlayerController::new(
				&entity_world,
				account_id,
				input_user.as_ref().unwrap(),
			)
			.arclocked(),
		);

		engine::window::Window::builder()
			.with_title("Crystal Sphinx")
			.with_size(1280.0, 720.0)
			.with_resizable(true)
			.with_application::<CrystalSphinx>()
			.with_clear_color([0.0, 0.0, 0.0, 1.0].into())
			.build(&mut engine)?;
		if let Some(mut render_chain) = engine.render_chain_write() {
			render_chain.set_render_pass_info(
				engine::asset::Loader::load_sync(&CrystalSphinx::get_asset_id("render_pass/root"))?
					.downcast::<engine::graphics::render_pass::Pass>()
					.unwrap()
					.as_graphics()?,
			);
		}
		let model_cache = graphics::voxel::model::Cache::new().arclocked();
		graphics::voxel::model::Load::start(&model_cache);
		/*
		{
			use graphics::voxel::{model::*, texture::*, Face};
			model_cache.insert(Model::new(std::collections::HashMap::from([
				(
					Face::Left,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
				(
					Face::Right,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
				(
					Face::Up,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
				(
					Face::Down,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
				(
					Face::Front,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
				(
					Face::Back,
					AtlasTexCoord::new([0.0, 0.0].into(), [1.0, 1.0].into()),
				),
			])));
		}
		*/

		let arc_camera = graphics::voxel::camera::ArcLockCamera::default();
		graphics::voxel::RenderVoxel::add_state_listener(
			&app_state,
			&engine.render_chain().unwrap(),
			&model_cache,
			&arc_camera,
		);
		graphics::chunk_boundary::Render::add_state_listener(
			&app_state,
			&engine.render_chain().unwrap(),
			&arc_camera,
			input_user.as_ref().unwrap(),
		);
		engine.add_system(entity::system::UpdateCamera::new(&entity_world, arc_camera).arclocked());

		let mut _egui_ui: Option<Arc<RwLock<engine::ui::egui::Ui>>> = None;
		#[cfg(feature = "debug")]
		{
			use engine::ui::egui::Ui;
			let command_list = commands::create_list(&app_state);
			let ui = Ui::create_with_subpass(
				&mut engine,
				Some(CrystalSphinx::get_asset_id("render_pass/egui_subpass").as_string()),
			)?;
			ui.write().unwrap().add_owned_element(
				debug::Panel::new(input_user.as_ref().unwrap())
					.with_window("Commands", debug::CommandWindow::new(command_list.clone()))
					.with_window(
						"Entity Inspector",
						debug::EntityInspector::new(&entity_world),
					)
					.with_window("Chunk Inspector", debug::ChunkInspector::new()),
			);
			_egui_ui = Some(ui);
		}

		let viewport = ui::AppStateViewport::new().arclocked();
		// initial UI is added when a callback matching the initial state is added to the app-state-machine
		ui::AppStateViewport::add_state_listener(&viewport, &app_state);

		// TEMPORARY: Emulate loading by causing a transition to the main menu after 3 seconds
		{
			let thread_app_state = app_state.clone();
			std::thread::spawn(move || {
				std::thread::sleep(std::time::Duration::from_secs(3));
				thread_app_state
					.write()
					.unwrap()
					.transition_to(app::state::State::MainMenu, None);
			});
		}

		{
			let ui_system = {
				use engine::ui::{oui::viewport, raui::make_widget};
				engine::ui::System::new(engine.render_chain().unwrap())?
					.with_engine_shaders()?
					.with_all_fonts()?
					//.with_tree_root(engine::ui::raui::make_widget!(ui::root::root))
					.with_tree_root(make_widget!(viewport::widget::<ui::AppStateViewport>))
					.with_context(viewport.clone())
					.with_texture(&CrystalSphinx::get_asset_id("textures/ui/title"))?
					.attach_system(
						&mut engine,
						Some(CrystalSphinx::get_asset_id("render_pass/ui_subpass").as_string()),
					)?
			};
			viewport.write().unwrap().set_system(&ui_system);
		}

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
	}

	log::info!(target: CrystalSphinx::name(), "Initialization finished");
	let engine = engine.into_arclock();
	engine::Engine::run(engine.clone(), || {
		if let Ok(mut guard) = account::ClientRegistry::write() {
			(*guard).logout();
		}
	})
}
