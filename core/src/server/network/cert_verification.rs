use std::{sync::Arc, time::SystemTime};

use rustls::server::{ClientCertVerified, ClientCertVerifier};

// Implementation of `ClientCertVerifier` that verifies everything as trustworthy.
pub struct AllowAnyClient;

impl AllowAnyClient {
	pub fn new() -> Arc<Self> {
		Arc::new(Self)
	}
}

impl ClientCertVerifier for AllowAnyClient {
	fn client_auth_root_subjects(&self) -> Option<rustls::DistinguishedNames> {
		Some(vec![])
	}

	fn verify_client_cert(
		&self,
		_end_entity: &rustls::Certificate,
		_intermediates: &[rustls::Certificate],
		_now: SystemTime,
	) -> Result<ClientCertVerified, rustls::Error> {
		log::info!(target: "server", "Ignoring verification of client certificate");
		Ok(ClientCertVerified::assertion())
	}
}
