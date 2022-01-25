use super::Instruction;
use crate::{
	app::{self, state::ArcLockMachine},
	common::network::ConnectionList,
	entity::{self, ArcLockEntityWorld},
	network::storage::{client::ArcLockClient, server::Server, ArcLockStorage},
};
use engine::{
	network::{self, endpoint::Endpoint, mode, stream, Config, LocalData},
	utility::Result,
};
use std::sync::{Arc, RwLock, Weak};

#[profiling::function]
pub fn load_dedicated_server(
	app_state: ArcLockMachine,
	storage: ArcLockStorage,
	entity_world: Weak<RwLock<entity::World>>,
) -> Result<()> {
	load_network(
		&storage,
		&entity_world,
		&Instruction {
			mode: mode::Kind::Server.into(),
			port: LocalData::get_named_arg("host_port"),
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
	storage: &ArcLockStorage,
	entity_world: &ArcLockEntityWorld,
) {
	use app::state::{State::*, Transition::*, *};
	for state in [LoadingWorld, Connecting].iter() {
		let callback_storage = storage.clone();
		let callback_entity_world = Arc::downgrade(&entity_world);
		app_state.write().unwrap().add_async_callback(
			OperationKey(None, Some(Enter), Some(*state)),
			move |operation| {
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
					let endpoint = load_network(&async_storage, &async_entity_world, &instruction)?;

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
					if instruction.mode == mode::Kind::Client {
						use stream::Initiator;
						let url = instruction.server_url.unwrap();
						let connection =
							endpoint.connect(url.parse()?, "server".to_owned()).await?;
						crate::common::network::Handshake::open(&connection)?;
					}

					Ok(())
				}
			},
		);
	}
}

#[profiling::function]
fn load_network(
	storage: &ArcLockStorage,
	entity_world: &Weak<RwLock<entity::World>>,
	instruction: &Instruction,
) -> Result<Arc<Endpoint>> {
	if instruction.mode.contains(mode::Kind::Server) {
		let world_name = instruction.world_name.as_ref().unwrap();
		if let Ok(server) = Server::load(&world_name) {
			storage.write().unwrap().set_server(server);
		}
	}
	if instruction.mode.contains(mode::Kind::Client) {
		storage
			.write()
			.unwrap()
			.set_client(ArcLockClient::default());
	}

	// let _ = crate::network::create(instruction.mode, &app_state, &storage, &entity_world)
	// 	.with_port(socknet_port)
	// 	.spawn();

	let socknet_port = instruction.port.unwrap_or(25565);
	let (endpoint, error_receiver) = {
		use std::net::{IpAddr, Ipv4Addr, SocketAddr};
		let endpoint_config = storage.read().unwrap().create_config()?;
		let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), socknet_port);
		let (error_sender, error_receiver) = async_channel::unbounded();
		let network_config = Config {
			endpoint: endpoint_config,
			address,
			stream_processor: Arc::new({
				use crate::common::network::*;
				use network::stream::processor::Registry;
				let mut registry = Registry::default();
				registry.register::<Handshake>();
				registry
			}),
			error_sender,
		};
		let endpoint = network_config.build()?;

		if let Ok(mut storage) = storage.write() {
			storage.set_endpoint(endpoint.clone());
		}

		(endpoint, error_receiver)
	};

	engine::task::spawn("network", async move {
		while let Ok(stream_error) = error_receiver.recv().await {
			log::error!(target: "network", "{}", stream_error);
		}
		Ok(())
	});

	if let Ok(mut storage) = storage.write() {
		storage.set_connection_list(ConnectionList::new(endpoint.connection_receiver().clone()));
		//storage.start_loading(&entity_world.upgrade().unwrap());
	}

	Ok(endpoint)
}
