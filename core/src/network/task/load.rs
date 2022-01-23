use super::{Directive, Instruction};
use crate::{
	account::key::Certificate,
	app::{self, state::ArcLockMachine},
	entity::{self, ArcLockEntityWorld},
	network::{
		packet::Handshake,
		storage::{client::ArcLockClient, server::Server, ArcLockStorage},
	},
};
use engine::{
	network::{self, endpoint, mode, Config, LocalData},
	task,
	utility::Result,
};
use std::sync::{Arc, RwLock, Weak};

#[profiling::function]
pub fn load_dedicated_server(
	app_state: ArcLockMachine,
	storage: ArcLockStorage,
	entity_world: Weak<RwLock<entity::World>>,
) {
	task::spawn_blocking("load-network", move || {
		load_network(
			&app_state,
			&storage,
			&entity_world,
			Instruction {
				mode: mode::Kind::Server.into(),
				port: LocalData::get_named_arg("host_port"),
				directive: Directive::LoadWorld("tmp".to_owned()),
			},
		)
	});
}

pub fn add_load_network_listener(
	app_state: &ArcLockMachine,
	storage: &ArcLockStorage,
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
					load_network(
						&async_app_state,
						&async_storage,
						&async_entity_world,
						instruction,
					)
				}
			},
		);
	}
}

#[profiling::function]
fn load_network(
	app_state: &ArcLockMachine,
	storage: &ArcLockStorage,
	entity_world: &Weak<RwLock<entity::World>>,
	instruction: Instruction,
) -> Result<()> {
	if instruction.mode.contains(mode::Kind::Server) {
		let world_name = match &instruction.directive {
			Directive::LoadWorld(world_name) => world_name,
			_ => unimplemented!(),
		};
		if let Ok(server) = Server::load(&world_name) {
			if let Ok(mut storage) = storage.write() {
				storage.set_server(server);
			}
		}
	}
	if instruction.mode.contains(mode::Kind::Client) {
		if let Ok(mut storage) = storage.write() {
			storage.set_client(ArcLockClient::default());
		}
	}

	let entity_world = entity_world.upgrade().unwrap();
	let socknet_port = instruction.port.unwrap_or(25565);

	// let _ = crate::network::create(instruction.mode, &app_state, &storage, &entity_world)
	// 	.with_port(socknet_port)
	// 	.spawn();
	// if let Ok(storage) = storage.read() {
	// 	storage.start_loading(&entity_world);
	// }

	let (endpoint, error_receiver) = {
		use std::net::{IpAddr, Ipv4Addr, SocketAddr};
		let endpoint_config = storage.read().unwrap().create_config()?;
		let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), socknet_port);
		let (error_sender, error_receiver) = async_channel::unbounded();
		let network_config = Config {
			endpoint: endpoint_config,
			address,
			stream_processor: Arc::new(
				network::stream::processor::handler::Registry::message_handler(),
			),
			error_sender,
		};
		(network_config.build()?, error_receiver)
	};

	engine::task::spawn("network", async move {
		while let Ok(stream_error) = error_receiver.recv().await {
			log::error!(target: "network", "{}", stream_error);
		}
		Ok(())
	});

	let connection_receiver = endpoint.connection_receiver().clone();
	engine::task::spawn("network", async move {
		while let Ok(connection) = connection_receiver.recv().await {
			if let Some(identity) = connection.peer_identity() {
				if let Ok(certs) = identity.downcast::<Vec<rustls::Certificate>>() {
					log::info!(target: "network", "connected to address={} identity={}", connection.remote_address(), Certificate::fingerprint(&certs[0]));
				}
			}
		}
		Ok(())
	});

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
		let url = if instruction.mode == mode::Kind::Client {
			match &instruction.directive {
				Directive::Connect(url) => url.clone(),
				_ => unimplemented!(),
			}
		} else {
			// for Cotos, the server url is the address we just initialized the network with
			format!("127.0.0.1:{}", socknet_port)
		};
		// if let Err(err) = Handshake::connect_to_server(&url) {
		// 	log::error!("{}", err);
		// }

		endpoint.connect(url.parse()?, "server".to_owned());
	}

	if let Ok(storage) = storage.write() {
		storage.set_endpoint(endpoint);
	}

	if let Some(state) = instruction.get_next_app_state() {
		app_state.write().unwrap().transition_to(state, None);
	}

	Ok(())
}
