use crate::{
	client::account,
	client::world::chunk::cache,
	common::account::key,
	network::storage::{self, Storage},
};
use engine::{network::endpoint, socknet::connection::Connection, utility::Result};
use std::sync::{Arc, RwLock, Weak};

pub type ArcLockClient = Arc<RwLock<Client>>;
/// Container class for all client data which is present when a user is connected to a game server.
pub struct Client {
	chunk_cache: cache::ArcLock,
}

impl Default for Client {
	fn default() -> Self {
		let chunk_cache = Arc::new(RwLock::new(cache::Cache::new()));
		Self { chunk_cache }
	}
}

impl Client {
	pub fn chunk_cache(&self) -> &cache::ArcLock {
		&self.chunk_cache
	}

	pub fn get_keys(&self) -> Result<(rustls::Certificate, rustls::PrivateKey)> {
		let certificate: rustls::Certificate;
		let private_key: rustls::PrivateKey;
		{
			let registry = account::Manager::read().unwrap();
			let account = registry.active_account()?;
			match account.key() {
				key::Key::Private(cert, key) => {
					certificate = cert.clone().into();
					private_key = key.clone().into();
				}
				key::Key::Public(_) => return Err(key::Error::InvalidPrivacyPublic)?,
			}
		}
		Ok((certificate, private_key))
	}

	pub fn get_server_connection(
		storage: &Weak<RwLock<Storage>>,
	) -> Result<Option<Weak<Connection>>> {
		use storage::Error::{FailedToReadStorage, InvalidConnectionList, InvalidStorage};
		let arc_storage = storage.upgrade().ok_or(InvalidStorage)?;
		let storage = arc_storage.read().map_err(|_| FailedToReadStorage)?;
		let arc_connection_list = storage.connection_list();
		let connection_list = arc_connection_list
			.read()
			.map_err(|_| InvalidConnectionList)?;
		Ok(connection_list.first().cloned())
	}
}

// Implementation of `ServerCertVerifier` that verifies everything as trustworthy.
pub struct SkipServerVerification;

impl SkipServerVerification {
	pub fn new() -> Arc<Self> {
		Arc::new(Self)
	}
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
	fn verify_server_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		server_name: &rustls::ServerName,
		_scts: &mut dyn Iterator<Item = &[u8]>,
		_ocsp_response: &[u8],
		_now: std::time::SystemTime,
	) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
		log::info!(target: "client", "Ignoring verification of server certificate from {:?}", server_name);
		Ok(rustls::client::ServerCertVerified::assertion())
	}
}
