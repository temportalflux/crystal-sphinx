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

use crate::{app::state::State::InGame, common::network::mode, graphics::ChainConfig};
use anyhow::Context;
use engine::{
	asset, graphics::Chain, task::PinFutureResultLifetime, ui::egui, utility::ValueSet,
	window::Window, Application, Engine, EventLoop,
};
use std::{
	path::PathBuf,
	sync::{Arc, RwLock, Weak},
};

pub mod client;
pub mod common;
pub mod server;

pub mod app;
pub mod block;
pub mod commands;
pub mod debug;
pub mod entity;
pub mod graphics;
pub mod input;
pub mod plugin;
pub mod ui;

pub struct CrystalSphinx();
impl Application for CrystalSphinx {
	fn name() -> &'static str {
		std::env!("CARGO_PKG_NAME")
	}
	fn version() -> semver::Version {
		semver::Version::parse(std::env!("CARGO_PKG_VERSION")).unwrap()
	}
}

pub struct Runtime {
	config: plugin::Config,
	systems: Arc<ValueSet>,
}

impl Runtime {
	fn get_network_mode() -> mode::Kind {
		let is_client = std::env::args().any(|arg| arg == "-client");
		let is_dedicated_server = std::env::args().any(|arg| arg == "-server");
		assert_ne!(is_client, is_dedicated_server);
		match (is_client, is_dedicated_server) {
			(true, false) => mode::Kind::Client,
			(false, true) => mode::Kind::Server,
			_ => unimplemented!(),
		}
	}

	pub fn new(config: plugin::Config) -> Self {
		let systems = ValueSet::new();
		systems.insert(Self::get_network_mode());

		let app_state = app::state::Machine::new(app::state::State::Launching).arclocked();
		systems.insert(app_state.clone());

		let world = entity::ArcLockEntityWorld::default();
		systems.insert(world.clone());

		entity::add_state_listener(&app_state, Arc::downgrade(&world));

		let network_storage = common::network::Storage::new(&app_state);
		systems.insert(network_storage.clone());
		common::network::task::add_unloading_state_listener(&app_state);
		entity::system::OwnedByConnection::add_state_listener(
			&app_state,
			Arc::downgrade(&network_storage),
			Arc::downgrade(&world),
		);
		entity::system::Replicator::add_state_listener(
			&app_state,
			Arc::downgrade(&network_storage),
			Arc::downgrade(&world),
		);

		Self { config, systems }
	}

	fn window(&self) -> Option<Arc<Window>> {
		self.systems.get_arc::<Window>()
	}

	fn app_mode(&self) -> mode::Kind {
		self.systems.get::<mode::Kind>().unwrap()
	}
}
impl engine::Runtime for Runtime {
	fn logging_path() -> PathBuf {
		let logid = std::env::args()
			.find_map(|arg| arg.strip_prefix("-logid=").map(|s| s.to_owned()))
			.unwrap();
		let mut log_path = std::env::current_dir().unwrap().to_path_buf();
		log_path.push(format!("{}_{}.log", CrystalSphinx::name(), logid));
		log_path
	}

	fn register_asset_types() {
		let mut registry = asset::TypeRegistry::get().write().unwrap();
		engine::register_asset_types(&mut registry);
		registry.register::<block::Block>();
		registry.register::<client::model::blender::Asset>();
	}

	fn initialize<'a>(&'a self, engine: Arc<RwLock<Engine>>) -> PinFutureResultLifetime<'a, bool> {
		Box::pin(async move {
			// Load bundled plugins so they can be used throughout the instance
			if let Ok(mut manager) = plugin::Manager::write() {
				manager.load(&self.config);
			}

			engine::asset::Library::scan_pak_directory()
				.await
				.context("scan paks")?;
			block::Lookup::initialize();
			entity::component::register_types();

			InGameSystems::add_state_listener(&self.systems);

			let app_state = self.systems.get_arclock::<app::state::Machine>().unwrap();
			if let Ok(mut engine) = engine.write() {
				engine.add_weak_system(Arc::downgrade(&app_state));
			}

			if self.app_mode() == mode::Kind::Server {
				let network_storage = self
					.systems
					.get_arclock::<common::network::Storage>()
					.unwrap();
				let world = self.systems.get_arclock::<entity::World>().unwrap();
				common::network::task::load_dedicated_server(
					app_state,
					network_storage,
					Arc::downgrade(&world),
				)
				.context("load_dedicated_server")?;
			}

			log::info!(target: CrystalSphinx::name(), "Initialization finished");
			Ok(true)
		})
	}

	fn create_display(
		&mut self,
		engine: &Arc<RwLock<Engine>>,
		event_loop: &EventLoop<()>,
	) -> anyhow::Result<()> {
		if self.app_mode() == mode::Kind::Server {
			return Ok(());
		}

		{
			let mut manager = client::account::Manager::write().unwrap();
			manager.scan_accounts().context("scan accounts")?;

			let user_name = std::env::args()
				.find_map(|arg| arg.strip_prefix("-user=").map(|s| s.to_owned()))
				.unwrap();

			let user_id = manager.ensure_account(&user_name)?;
			manager.login_as(&user_id).context("login account")?;
		};

		let input_user = input::init();

		let graphics_chain = {
			let window = Window::builder()
				.with_title("Crystal Sphinx")
				.with_size(1280.0, 720.0)
				.with_resizable(true)
				.with_application::<CrystalSphinx>()
				.build(event_loop)
				.context("build window")?;
			let graphics_chain = window.graphics_chain().clone();
			self.systems.insert(Arc::new(window));
			self.systems.insert(Arc::downgrade(&graphics_chain));
			graphics_chain
		};

		let render_phases = {
			let mut chain = graphics_chain.write().unwrap();
			chain
				.apply_procedure::<ChainConfig>()
				.context("apply render procedure")?
		};

		// TODO: wait for the thread to finish before allowing the user in the world.
		let arc_camera = graphics::voxel::camera::ArcLockCamera::default();

		self.systems.insert(render_phases.clone());
		self.systems.insert(arc_camera.clone());
		self.systems.insert(input_user.clone());

		let app_state = self.systems.get_arclock::<app::state::Machine>().unwrap();
		let network_storage = self
			.systems
			.get_arclock::<common::network::Storage>()
			.unwrap();
		let world = self.systems.get_arclock::<entity::World>().unwrap();
		common::network::task::add_load_network_listener(&app_state, &network_storage, &world);

		entity::system::PlayerController::add_state_listener(
			&app_state,
			Arc::downgrade(&network_storage),
			Arc::downgrade(&world),
			input_user.clone(),
		);

		let fn_view_world = Arc::downgrade(&world);
		let fn_view_input = input_user.clone();
		app::store_during(&app_state, InGame, move || {
			client::UpdateCameraView::create(fn_view_world.clone(), &fn_view_input)
		});

		graphics::voxel::model::load_models(
			&app_state,
			Arc::downgrade(&network_storage),
			&graphics_chain,
			&render_phases.world,
			&arc_camera,
			&world,
		);

		if let Ok(mut engine) = engine.write() {
			engine.add_system(entity::system::UpdateCamera::new(&world, arc_camera).arclocked());
		}

		#[cfg(feature = "debug")]
		{
			let command_list = commands::create_list(&app_state);
			let ui = egui::Ui::create(&self.window().unwrap(), &*event_loop, &render_phases.egui)
				.context("create debug ui")?;
			ui.write().unwrap().add_owned_element(
				debug::Panel::new(&input_user)
					.with_window("Commands", debug::CommandWindow::new(command_list.clone()))
					.with_window("Entity Inspector", debug::EntityInspector::new(&world))
					.with_window("Chunk Inspector", debug::ChunkInspector::new()),
			);
			if let Ok(mut engine) = engine.write() {
				engine.add_winit_listener(&ui);
			}
			self.systems.insert(ui);
		}

		let viewport = ui::AppStateViewport::new().arclocked();
		// initial UI is added when a callback matching the initial state is added to the app-state-machine
		ui::AppStateViewport::add_state_listener(&viewport, &app_state);

		// TEMPORARY: Emulate loading by causing a transition to the main menu after 3 seconds
		{
			let thread_app_state = app_state.clone();
			engine::task::spawn("temp".to_owned(), async move {
				tokio::time::sleep(std::time::Duration::from_secs(3)).await;
				thread_app_state
					.write()
					.unwrap()
					.transition_to(app::state::State::MainMenu, None);
				Ok(())
			});
		}

		{
			let ui_system = {
				use engine::ui::{oui::viewport, raui::make_widget};
				let mut engine = engine.write().unwrap();
				engine::ui::System::new(&graphics_chain)?
					.with_engine_shaders()?
					.with_all_fonts()?
					//.with_tree_root(engine::ui::raui::make_widget!(ui::root::root))
					.with_tree_root(make_widget!(viewport::widget::<ui::AppStateViewport>))
					.with_context(viewport.clone())
					.with_texture(&CrystalSphinx::get_asset_id("textures/ui/title"))?
					.attach_system(&mut engine, &graphics_chain, &render_phases.ui)?
			};
			viewport.write().unwrap().set_system(&ui_system);
		}

		Ok(())
	}

	fn get_display_chain(&self) -> Option<Arc<RwLock<Chain>>> {
		self.window().map(|window| window.graphics_chain().clone())
	}

	fn on_event_loop_complete(&self) {
		// Make sure any app-state storages are cleared out before the window is destroyed (to ensure render objects are dropped in the correct order).
		let app_state = self.systems.get_arclock::<app::state::Machine>().unwrap();
		if let Ok(mut app_state) = app_state.write() {
			app_state.clear_callbacks();
		}
		if let Ok(mut guard) = client::account::Manager::write() {
			(*guard).logout();
		}
	}
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

pub struct InGameSystems {
	systems: Arc<ValueSet>,
}
impl InGameSystems {
	pub fn add_state_listener(systems: &Arc<ValueSet>) {
		use app::state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let app_state = systems.get_arclock::<app::state::Machine>().unwrap();
		let callback_systems = Arc::downgrade(&systems);
		Storage::<Self>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				profiling::scope!("init-game-systems");
				let in_game = Self {
					systems: callback_systems.upgrade().unwrap(),
				};
				in_game.add_common_systems();
				if mode::get().contains(mode::Kind::Client) {
					in_game.add_client_systems()?;
				}
				Ok(Some(in_game))
			});
	}

	#[profiling::function]
	fn add_common_systems(&self) {
		// Both clients and servers run the physics simulation.
		// The server will broadcast authoritative values (via components marked as `Replicatable`),
		// and clients will tell the server of the changes to the entities they own via TBD.
		let world = self.systems.get_arclock::<entity::World>().unwrap();
		let old_physics = common::physics::SimplePhysics::new(&world).arclocked();
		let physics = common::physics::System::new(&world).arclocked();
		{
			let mut engine = Engine::get().write().unwrap();
			engine.add_weak_system(Arc::downgrade(&old_physics));
			engine.add_weak_system(Arc::downgrade(&physics));
		}
		self.systems.insert(old_physics);
		self.systems.insert(physics);
	}

	#[profiling::function]
	fn add_client_systems(&self) -> anyhow::Result<()> {
		let arc_chain = self
			.systems
			.get_weaklock_upgraded::<engine::graphics::Chain>()
			.unwrap();

		let camera = self
			.systems
			.get_arclock::<graphics::voxel::camera::Camera>()
			.unwrap();
		let input_user = self.systems.get_arclock::<input::User>().unwrap();
		let render_phases = self.systems.get::<graphics::Phases>().unwrap();

		let action_toggle_chunk_boundaries =
			input::User::get_action_in(&input_user, crate::input::ACTION_TOGGLE_CHUNK_BOUNDARIES);

		let render_chunk_boundaries = graphics::chunk_boundary::Render::new(
			&arc_chain.read().unwrap(),
			camera.clone(),
			action_toggle_chunk_boundaries.unwrap(),
		)?
		.arclocked();

		log::debug!("create collider systems");

		let (gather_renderable_colliders, render_colliders) =
			client::physics::create_collider_systems(&self.systems)?;
		{
			let mut engine = Engine::get().write().unwrap();
			engine.add_weak_system(Arc::downgrade(&gather_renderable_colliders));
		}

		log::debug!("collider systems created");

		{
			profiling::scope!("attach chain operations");
			let mut chain = arc_chain.write().unwrap();
			chain.add_operation(
				&render_phases.debug,
				Arc::downgrade(&render_chunk_boundaries),
				None,
			)?;
			chain.add_operation(
				&render_phases.debug,
				Arc::downgrade(&render_colliders),
				None,
			)?;
		}

		self.systems.insert(render_chunk_boundaries);
		self.systems.insert(gather_renderable_colliders);
		self.systems.insert(render_colliders);

		Ok(())
	}
}

impl Drop for InGameSystems {
	fn drop(&mut self) {
		// Common
		self.systems
			.remove::<Arc<RwLock<common::physics::SimplePhysics>>>();
		self.systems
			.remove::<Arc<RwLock<common::physics::System>>>();
		// Client
		self.systems
			.remove::<Arc<RwLock<graphics::chunk_boundary::Render>>>();
		self.systems
			.remove::<Arc<RwLock<client::physics::GatherRenderableColliders>>>();
		self.systems
			.remove::<Arc<RwLock<client::physics::RenderColliders>>>();
	}
}
