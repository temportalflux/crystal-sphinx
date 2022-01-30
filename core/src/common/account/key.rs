use crate::common::utility::DataFile;
use engine::utility::Result;
use std::path::Path;

pub fn create_pem() -> Result<(String, String, String)> {
	// TODO: This should eventually use a third-party certificate distributer
	// (https://quinn-rs.github.io/quinn/quinn/certificate.html).
	// Default algo: rcgen::PKCS_ECDSA_P256_SHA256
	let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
	let certificate = cert.serialize_pem()?;
	let fingerprint = Certificate::from_pem(certificate.clone())?.fingerprint();
	let private_key = cert.serialize_private_key_pem();
	Ok((
		fingerprint,
		certificate,
		private_key,
	))
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

#[derive(Clone)]
pub enum Key {
	Private(Certificate, PrivateKey),
	Public(PublicKey),
}

impl std::fmt::Debug for Key {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Private(_, _) => write!(f, "Private"),
			Self::Public(PublicKey(encoded)) => write!(f, "Public({})", encoded),
		}
	}
}

#[derive(Clone)]
pub struct Certificate(Vec<u8>);

impl DataFile for Certificate {
	fn file_name() -> &'static str {
		"certificate.pem"
	}

	fn save_to(&self, _file_path: &Path) -> Result<()> {
		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		let pem = std::fs::read_to_string(&file_path)?;
		Self::from_pem(pem)
	}
}

impl Certificate {
	pub fn from_pem(pem: String) -> Result<Self> {
		let bytes = parse_pem(pem).ok_or(Error::InvalidPEM)?;
		Ok(Self(bytes))
	}

	pub fn fingerprint(&self) -> String {
		use engine::network::socknet::utility::fingerprint;
		fingerprint(&self.clone().into())
	}
}

impl Into<rustls::Certificate> for Certificate {
	fn into(self) -> rustls::Certificate {
		rustls::Certificate(self.0)
	}
}

#[derive(Clone)]
pub struct PrivateKey(Vec<u8>);

impl DataFile for PrivateKey {
	fn file_name() -> &'static str {
		"private_key.pem"
	}

	fn save_to(&self, _file_path: &Path) -> Result<()> {
		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		let pem = std::fs::read_to_string(&file_path)?;
		let bytes = parse_pem(pem).ok_or(Error::InvalidPEM)?;
		Ok(Self(bytes))
	}
}

impl Into<rustls::PrivateKey> for PrivateKey {
	fn into(self) -> rustls::PrivateKey {
		rustls::PrivateKey(self.0)
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct PublicKey(/*base64 encoded bytes*/ String);

impl DataFile for PublicKey {
	fn file_name() -> &'static str {
		"public_key.pem"
	}

	fn save_to(&self, file_path: &Path) -> Result<()> {
		std::fs::write(&file_path, self.0.clone())?;
		Ok(())
	}

	fn load_from(file_path: &Path) -> Result<Self> {
		Ok(Self(std::fs::read_to_string(&file_path)?))
	}
}

impl PublicKey {
	pub fn from_bytes(bytes: Vec<u8>) -> Self {
		use engine::network::socknet::utility::encode_string;
		Self(encode_string(&bytes))
	}

	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		use engine::network::socknet::utility::decode_bytes;
		decode_bytes(&self.0)
	}
}

impl std::fmt::Display for PublicKey {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "PublicKey({})", self.0)
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("PEM file does not contain a x509 certificate or PKCS#8/RFC5958 private key.")]
	InvalidPEM,
	#[error("Expected private key, but found public key")]
	InvalidPrivacyPublic,
	#[error("Expected public key, but found private key")]
	InvalidPrivacyPrivate,
}
