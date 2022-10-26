use crate::{
	app::state::ArcLockMachine, common::network::connection, common::network::mode,
	entity::ArcLockEntityWorld,
};
use anyhow::Result;
use socknet::endpoint::{Config, Endpoint};
use std::sync::{Arc, RwLock};

pub type ArcLockStorage = Arc<RwLock<Storage>>;
type ArcLockClient = Arc<RwLock<crate::client::network::Storage>>;
type ArcLockServer = Arc<RwLock<crate::server::network::Storage>>;

#[derive(Default)]
pub struct Storage {
	client: Option<ArcLockClient>,
	server: Option<ArcLockServer>,
	endpoint: Option<Arc<Endpoint>>,
	connection_list: Option<Arc<RwLock<connection::List>>>,
}

impl Storage {
	pub fn new(app_state: &ArcLockMachine) -> ArcLockStorage {
		use crate::app::state::{State::*, Transition::*, *};

		let arclocked = Arc::new(RwLock::new(Self::default()));

		// Add callback to clear the client storage if the client disconnects
		{
			let callback_storage = arclocked.clone();
			let callback_app_state = Arc::downgrade(&app_state);
			app_state.write().unwrap().add_callback(
				OperationKey(None, Some(Enter), Some(Disconnecting)),
				move |_operation| {
					assert!(mode::get() == mode::Kind::Client);
					mode::set(mode::Set::empty());
					if let Ok(mut storage) = callback_storage.write() {
						storage.client = None;
						storage.endpoint = None;
						storage.connection_list = None;
					}

					let async_app_state = callback_app_state.clone();
					engine::task::spawn("disconnecting".to_owned(), async move {
						profiling::scope!("finalize-disconnect");
						let app_state = async_app_state.upgrade().unwrap();
						// This will be blocked for some ms until after the transition is complete,
						// because this callback is being performed via a mutable app_state.
						if let Ok(mut app_state) = app_state.write() {
							app_state.transition_to(MainMenu, None);
						}
						Ok(())
					});
				},
			);
		}

		// Add callback to clear the server storage if the server unloads
		{
			let callback_storage = arclocked.clone();
			app_state.write().unwrap().add_callback(
				OperationKey(None, Some(Enter), Some(Unloading)),
				move |_operation| {
					profiling::scope!("unloading-network");
					assert!(mode::get().contains(mode::Kind::Server));
					mode::set(mode::Set::empty());
					if let Ok(mut storage) = callback_storage.write() {
						storage.server = None;
						// Clear out client if it was integrated
						storage.client = None;
						storage.endpoint = None;
						storage.connection_list = None;
					}
				},
			);
		}

		arclocked
	}

	pub fn arclocked(self) -> ArcLockStorage {
		Arc::new(RwLock::new(self))
	}

	pub fn set_server(&mut self, server: crate::server::network::Storage) {
		self.server = Some(Arc::new(RwLock::new(server)));
	}

	pub fn server(&self) -> &Option<ArcLockServer> {
		&self.server
	}

	pub fn set_client(&mut self, client: ArcLockClient) {
		self.client = Some(client);
	}

	pub fn client(&self) -> &Option<ArcLockClient> {
		&self.client
	}

	pub fn create_config(&self) -> Result<Config> {
		use socknet::endpoint;

		// If this is a client (regardless of also being a server or not),
		// use the clients certifications.
		let (certificate, private_key) = match (self.client.as_ref(), self.server.as_ref()) {
			(Some(client), _) => client.read().unwrap().get_keys()?,
			(None, Some(server)) => server.read().unwrap().get_keys()?,
			(None, None) => unimplemented!(),
		};

		// Integrated & Dedicated servers both use the ServerConfig route
		if self.server.is_some() {
			use crate::server::network::AllowAnyClient;
			let crypto_config = rustls::ServerConfig::builder()
				.with_safe_defaults()
				.with_client_cert_verifier(AllowAnyClient::new())
				.with_single_cert(vec![certificate.clone()], private_key.clone())?;
			let quinn_config = quinn::ServerConfig::with_crypto(Arc::new(crypto_config));
			Ok(Config::Server(endpoint::ServerConfig {
				core: quinn_config,
				certificate,
				private_key,
			}))
		} else {
			use crate::client::network::SkipServerVerification;
			let crypto_config = rustls::ClientConfig::builder()
				.with_safe_defaults()
				.with_custom_certificate_verifier(SkipServerVerification::new())
				.with_single_cert(vec![certificate.clone()], private_key.clone())?;

			let mut transport_config = quinn::TransportConfig::default();
			transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));

			let mut quinn_config = quinn::ClientConfig::new(Arc::new(crypto_config));
			quinn_config.transport = Arc::new(transport_config);

			Ok(Config::Client(endpoint::ClientConfig {
				core: quinn_config,
				certificate,
				private_key,
			}))
		}
	}

	pub fn set_endpoint(&mut self, endpoint: Arc<Endpoint>) {
		self.endpoint = Some(endpoint);
	}

	pub fn set_connection_list(&mut self, list: Arc<RwLock<connection::List>>) {
		self.connection_list = Some(list);
	}

	pub fn connection_list(&self) -> &Arc<RwLock<connection::List>> {
		self.connection_list.as_ref().unwrap()
	}

	pub fn start_loading(&self, entity_world: &ArcLockEntityWorld) -> anyhow::Result<()> {
		if let Some(arc_server) = self.server.as_ref() {
			if let Ok(mut server) = arc_server.write() {
				server.start_loading_world()?;
				server.initialize_systems(&entity_world);
			}
		}
		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("storage is invalid")]
	InvalidStorage,
	#[error("failed to read from storage data")]
	FailedToReadStorage,

	#[error("server storage is invalid")]
	InvalidServer,
	#[error("failed to read from server data")]
	FailedToReadServer,
	#[error("failed to write to server data")]
	FailedToWriteServer,

	#[error("client storage is invalid")]
	InvalidClient,
	#[error("failed to read from client data")]
	FailedToReadClient,
	#[error("failed to write to client data")]
	FailedToWriteClient,

	#[error("connection list is invalid")]
	InvalidConnectionList,
}
