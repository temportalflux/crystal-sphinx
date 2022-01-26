use crate::common::utility::DataFile;
use engine::utility::Result;
use std::path::Path;

pub fn new() -> Result<(Certificate, PrivateKey)> {
	// TODO: This should eventually use a third-party certificate distributer
	// (https://quinn-rs.github.io/quinn/quinn/certificate.html).
	// Default algo: rcgen::PKCS_ECDSA_P256_SHA256
	let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
	let private_key = PrivateKey(cert.serialize_private_key_pem());
	let certificate = Certificate(cert.serialize_pem()?);
	Ok((certificate, private_key))
}

fn parse_pem(pem: String) -> Option<Vec<u8>> {
	use rustls_pemfile::{read_one, Item};
	use std::iter;
	let mut reader = std::io::BufReader::new(std::io::Cursor::new(pem));
	for item in iter::from_fn(|| read_one(&mut reader).transpose()) {
		match item.unwrap() {
			Item::X509Certificate(cert) => return Some(cert),
			Item::PKCS8Key(key) => return Some(key),
			Item::RSAKey(_) => {} // no-op
		}
	}
	None
}

#[derive(Debug, Clone)]
pub struct Certificate(String);

impl DataFile for Certificate {
	fn file_name() -> &'static str {
		"certificate.pem"
	}

	fn save_to(&self, file_path: &Path) -> Result<()> {
		std::fs::write(&file_path, self.0.clone())?;
		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		Ok(Self(std::fs::read_to_string(&file_path)?))
	}
}

impl Certificate {
	pub fn serialized(&self) -> Result<rustls::Certificate> {
		let der_bytes = parse_pem(self.0.clone()).ok_or(InvalidPEM)?;
		Ok(rustls::Certificate(der_bytes))
	}
}

#[derive(Debug, Clone)]
pub struct PrivateKey(String);

impl DataFile for PrivateKey {
	fn file_name() -> &'static str {
		"private_key.pem"
	}

	fn save_to(&self, file_path: &Path) -> Result<()> {
		std::fs::write(&file_path, self.0.clone())?;
		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		Ok(Self(std::fs::read_to_string(&file_path)?))
	}
}

impl PrivateKey {
	pub fn serialized(&self) -> Result<rustls::PrivateKey> {
		let der_bytes = parse_pem(self.0.clone()).ok_or(InvalidPEM)?;
		Ok(rustls::PrivateKey(der_bytes))
	}
}

struct InvalidPEM;
impl std::error::Error for InvalidPEM {}
impl std::fmt::Debug for InvalidPEM {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for InvalidPEM {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"PEM file does not contain a x509 certificate or PKCS#8/RFC5958 private key."
		)
	}
}
