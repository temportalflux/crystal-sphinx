use engine::utility::AnyError;
use std::convert::TryFrom;

/// Trait implemented by components to mark that the component is replicated to relevant connections.
pub trait Replicated
where
	Self: TryFrom<Vec<u8>>,
{
	fn unique_id() -> &'static str;

	fn serialize(&self) -> Result<Vec<u8>, AnyError>;
}

pub fn deserialize<'a, T>(bytes: &'a Vec<u8>) -> Result<T, rmp_serde::decode::Error>
where
	T: serde::Deserialize<'a>,
{
	rmp_serde::from_read_ref::<'a, Vec<u8>, T>(&bytes)
}
