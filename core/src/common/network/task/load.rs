use super::Instruction;
use crate::{
	app,
	common::{
		network::{connection, mode},
		utility::get_named_arg,
		world,
	},
	entity,
	server::network::Storage as ServerStorage,
};
use anyhow::{Context, Result};
use engine::utility::ValueSet;
use socknet::{endpoint::Endpoint, Config};
use std::sync::{Arc, RwLock};

#[profiling::function]
pub fn load_dedicated_server(systems: Arc<ValueSet>) -> Result<()> {
	let app_state = systems.get_arclock::<app::state::Machine>().unwrap();
	load_network(
		systems,
		&Instruction {
			mode: mode::Kind::Server.into(),
			port: get_named_arg("host_port"),
			world_name: Some("tmp".to_owned()),
			server_url: None,
		},
	)?;
	app_state
		.write()
		.unwrap()
		.transition_to(crate::app::state::State::InGame, None);
	Ok(())
}

pub fn add_load_network_listener(systems: &Arc<ValueSet>) {
	use app::state::{State::*, Transition::*, *};
	let app_state = systems.get_arclock::<app::state::Machine>().unwrap();

	// Construct world systems when entering the world
	for state in [LoadingWorld, Connecting].iter() {
		let callback_systems = Arc::downgrade(systems);
		app_state.write().unwrap().add_async_callback(
			OperationKey(None, Some(Enter), Some(*state)),
			move |operation| {
				let async_systems = callback_systems.clone();
				let instruction = operation
					.data()
					.as_ref()
					.unwrap()
					.downcast_ref::<Instruction>()
					.unwrap()
					.clone();
				async move {
					let Some(systems) = async_systems.upgrade() else {
						return Ok(());
					};

					// Add the world database whenever the world is loaded, client and server alike.
					systems.insert(Arc::new(RwLock::new(world::Database::new())));

					let endpoint = load_network(systems, &instruction)?;

					// Dedicated Client (mode == Client) needs to connect to the server.
					// Additionally... Integrated Client-Server (mode == Client + Server) should run
					// authentication against its save data.
					// We can't prevent a smart user from downloading save data and replacing the
					// user id's and public key with their own, but we can at least do a bare-bones
					// "your id and public keys match" authentication.
					// Really we want to run the auth flow here because it allows us to put
					// initialization for entities on the server in the handshake and
					// initialization for entities on the client in the replication packet,
					// running both for Integrated Client-Server/Client-on-top-of-Server.
					if instruction.mode.contains(mode::Kind::Client) {
						use crate::common::network::handshake::client::Handshake;
						use socknet::stream::handler::Initiator;
						let url = match instruction.mode == mode::Kind::Client {
							true => instruction.server_url.unwrap().parse()?,
							false => endpoint.address(),
						};
						let connection = endpoint.connect(url, "server".to_owned()).await?;
						Handshake::open(&connection)?.await?.initiate();
					}

					Ok(())
				}
			},
		);
	}

	// Deconstruct world systems when leaving the world
	for state in [Unloading, Disconnecting].iter() {
		let callback_systems = Arc::downgrade(systems);
		app_state.write().unwrap().add_async_callback(
			OperationKey(None, Some(Enter), Some(*state)),
			move |_operation| {
				let async_systems = callback_systems.clone();
				async move {
					let Some(systems) = async_systems.upgrade() else {
						return Ok(());
					};

					// Both
					systems.remove::<Arc<world::Database>>();
					// Server-Only
					systems.remove::<Arc<crate::server::world::Loader>>();
					// Client-Only
					systems.remove::<Arc<crate::client::world::ChunkChannel>>();

					Ok(())
				}
			},
		);
	}
}

#[profiling::function]
fn load_network(systems: Arc<ValueSet>, instruction: &Instruction) -> Result<Arc<Endpoint>> {
	mode::set(instruction.mode.clone());

	let app_state = systems.get_arclock::<app::state::Machine>().unwrap();
	let storage = systems
		.get_arclock::<crate::common::network::Storage>()
		.unwrap();
	let entity_world = Arc::downgrade(&systems.get_arclock::<entity::World>().unwrap());
	let database = systems.get_arclock::<world::Database>().unwrap();

	if instruction.mode.contains(mode::Kind::Server) {
		let world_name = instruction.world_name.as_ref().unwrap();
		let server = ServerStorage::load(&world_name).context("loading server")?;

		systems.insert(Arc::new(crate::server::world::Loader::new(
			server.world_path(),
			Arc::downgrade(&database),
		)?));

		storage.write().unwrap().set_server(server);
	}
	if instruction.mode.contains(mode::Kind::Client) {
		storage.write().unwrap().set_client(Default::default());

		// Add the async task to funnel world updates to the client channel
		let recv_updates = database.write().unwrap().add_recv();
		systems.insert(Arc::new(crate::client::world::ChunkChannel::new(
			recv_updates,
		)));
	}

	let socknet_port = instruction.port.unwrap_or(25565);
	let endpoint = {
		use std::net::{IpAddr, Ipv4Addr, SocketAddr};
		let endpoint_config = storage.read().unwrap().create_config()?;
		let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), socknet_port);
		let network_config = Config {
			endpoint: endpoint_config,
			address,
			stream_registry: Arc::new({
				use crate::common::network::*;
				use socknet::stream::Registry;
				let mut registry = Registry::default();
				registry.register(handshake::Identifier {
					client: Arc::new(handshake::client::AppContext {
						app_state: Arc::downgrade(&app_state),
					}),
					server: Arc::new(handshake::server::AppContext {
						storage: Arc::downgrade(&storage),
						entity_world: entity_world.clone(),
					}),
				});
				registry.register(client_joined::Identifier::default());
				registry.register(replication::entity::Identifier {
					server: Arc::default(),
					client: Arc::new(replication::entity::client::AppContext {
						entity_world: entity_world.clone(),
					}),
				});
				replication::world::register(&mut registry, &systems);
				registry.register(move_player::Identifier {
					client: Arc::default(),
					server: Arc::new(move_player::server::AppContext {
						entity_world: entity_world.clone(),
						sequencer: Default::default(),
					}),
				});
				registry
			}),
		};
		let endpoint = network_config.build()?;

		if let Ok(mut storage) = storage.write() {
			storage.set_endpoint(endpoint.clone());
		}

		endpoint
	};

	if let Ok(mut storage) = storage.write() {
		storage.set_connection_list(connection::List::new(
			endpoint.connection_receiver().clone(),
		));
		storage.start_loading(&systems)?;
	}

	Ok(endpoint)
}
