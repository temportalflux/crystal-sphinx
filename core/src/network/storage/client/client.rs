use crate::{account, client::world::chunk::cache};
use engine::{network::endpoint, utility::Result};
use std::sync::{Arc, RwLock};

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

	pub fn create_config(&self) -> Result<endpoint::ClientConfig> {
		let (certificate, private_key) = {
			let registry = account::ClientRegistry::read().unwrap();
			let account = registry
				.active_account()
				.ok_or(account::NoAccountLoggedIn)?;
			account.serialized_keys()?
		};

		let core_config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_custom_certificate_verifier(SkipServerVerification::new())
			.with_single_cert(vec![certificate.clone()], private_key.clone())?;
		Ok(endpoint::ClientConfig {
			core: quinn::ClientConfig::new(Arc::new(core_config)),
			certificate,
			private_key,
		})
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
