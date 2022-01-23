use crate::{account, client::world::chunk::cache};
use engine::{utility::Result, network::endpoint::Endpoint};
use std::sync::{Arc, RwLock};

pub type ArcLockClient = Arc<RwLock<Client>>;
/// Container class for all client data which is present when a user is connected to a game server.
pub struct Client {
	chunk_cache: cache::ArcLock,
	endpoint: Option<Endpoint>,
}

impl Default for Client {
	fn default() -> Self {
		let chunk_cache = Arc::new(RwLock::new(cache::Cache::new()));
		Self { chunk_cache, endpoint: None, }
	}
}

impl Client {
	pub fn chunk_cache(&self) -> &cache::ArcLock {
		&self.chunk_cache
	}

	pub fn create_config(&self) -> Result<quinn::ClientConfig> {
		let (cert, key) = {
			let registry = account::ClientRegistry::read().unwrap();
			let account = registry
				.active_account()
				.ok_or(account::NoAccountLoggedIn)?;
			account.serialized_keys()?
		};

		log::debug!(target: "client", "local identity={}", account::key::Certificate::fingerprint(&cert));

		let core_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(SkipServerVerification::new())
			.with_single_cert(vec![cert], key)?;
		Ok(quinn::ClientConfig::new(Arc::new(core_config)))
	}

	pub fn set_endpoint(&mut self, endpoint: Endpoint) {
		self.endpoint = Some(endpoint);
	}
}

// Implementation of `ServerCertVerifier` that verifies everything as trustworthy.
struct SkipServerVerification;

impl SkipServerVerification {
	fn new() -> Arc<Self> {
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
