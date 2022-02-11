use super::Instruction;
use crate::{
	app::{self, state::ArcLockMachine},
	common::{
		network::{connection, mode, Storage},
		utility::get_named_arg,
	},
	entity::{self, ArcLockEntityWorld},
	server::network::Storage as ServerStorage,
};
use engine::utility::{Context, Result};
use socknet::{endpoint::Endpoint, Config};
use std::sync::{Arc, RwLock, Weak};

#[profiling::function]
pub fn load_dedicated_server(
	app_state: ArcLockMachine,
	storage: Arc<RwLock<Storage>>,
	entity_world: Weak<RwLock<entity::World>>,
) -> Result<()> {
	load_network(
		&app_state,
		&storage,
		&entity_world,
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

pub fn add_load_network_listener(
	app_state: &ArcLockMachine,
	storage: &Arc<RwLock<Storage>>,
	entity_world: &ArcLockEntityWorld,
) {
	use app::state::{State::*, Transition::*, *};
	for state in [LoadingWorld, Connecting].iter() {
		let callback_app_state = app_state.clone();
		let callback_storage = storage.clone();
		let callback_entity_world = Arc::downgrade(&entity_world);
		app_state.write().unwrap().add_async_callback(
			OperationKey(None, Some(Enter), Some(*state)),
			move |operation| {
				let async_app_state = callback_app_state.clone();
				let async_storage = callback_storage.clone();
				let async_entity_world = callback_entity_world.clone();
				let instruction = operation
					.data()
					.as_ref()
					.unwrap()
					.downcast_ref::<Instruction>()
					.unwrap()
					.clone();
				async move {
					let endpoint = load_network(
						&async_app_state,
						&async_storage,
						&async_entity_world,
						&instruction,
					)?;

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
						use crate::common::network::Handshake;
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
}

#[profiling::function]
fn load_network(
	app_state: &ArcLockMachine,
	storage: &Arc<RwLock<Storage>>,
	entity_world: &Weak<RwLock<entity::World>>,
	instruction: &Instruction,
) -> Result<Arc<Endpoint>> {
	mode::set(instruction.mode.clone());

	if instruction.mode.contains(mode::Kind::Server) {
		let world_name = instruction.world_name.as_ref().unwrap();
		let server = ServerStorage::load(&world_name).context("loading server")?;
		storage.write().unwrap().set_server(server);
	}
	if instruction.mode.contains(mode::Kind::Client) {
		storage.write().unwrap().set_client(Default::default());
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
				registry.register(handshake::Builder {
					storage: Arc::downgrade(&storage),
					app_state: Arc::downgrade(&app_state),
					entity_world: entity_world.clone(),
				});
				registry.register(ClientJoined {});
				registry.register(replication::entity::Builder {
					entity_world: entity_world.clone(),
				});
				replication::world::register(&mut registry, Arc::downgrade(&storage));
				registry.register(move_player::Builder {
					entity_world: entity_world.clone(),
					sequencer: Default::default(),
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
		storage.start_loading(&entity_world.upgrade().unwrap());
	}

	Ok(endpoint)
}
