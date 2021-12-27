use engine::utility::{AnyError, VoidResult};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize)]
pub struct SerializedEntity {
	pub entity: hecs::Entity,
	pub components: Vec<SerializedComponent>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializedComponent {
	pub(crate) id: String,
	pub(crate) data: Vec<u8>,
}

/// Trait implemented by components to mark that the component is replicated to relevant connections.
pub trait Serializable: super::Component
where
	Self: TryFrom<Vec<u8>>,
{
	fn serialize(&self) -> Result<Vec<u8>, AnyError>;
}

pub fn deserialize<'a, T>(bytes: &'a Vec<u8>) -> Result<T, rmp_serde::decode::Error>
where
	T: serde::Deserialize<'a>,
{
	rmp_serde::from_read_ref::<'a, Vec<u8>, T>(&bytes)
}

pub struct Registration {
	serialize: Box<dyn Fn(&hecs::EntityRef<'_>) -> Result<Option<SerializedComponent>, AnyError>>,
	deserialize: Box<dyn Fn(Vec<u8>, &mut hecs::EntityBuilder) -> VoidResult>,
}

impl Registration {
	pub(crate) fn from<T>() -> Self
	where
		T: super::Component + Serializable,
	{
		Self {
			serialize: Box::new(
				|e: &hecs::EntityRef<'_>| -> Result<Option<SerializedComponent>, AnyError> {
					let data = match e.get::<T>() {
						Some(t_comp) => {
							profiling::scope!("serialize-component", T::unique_id());
							t_comp.serialize()
						}
						None => return Ok(None),
					}
					.map_err(|_| FailedToSerialize(T::unique_id()))?;
					Ok(Some(SerializedComponent {
						id: T::unique_id().to_owned(),
						data,
					}))
				},
			),
			deserialize: Box::new(
				|bytes: Vec<u8>, builder: &mut hecs::EntityBuilder| -> VoidResult {
					profiling::scope!("deserialize-component", T::unique_id());
					let comp =
						T::try_from(bytes).map_err(|_| FailedToDeserialize(T::unique_id()))?;
					builder.add(comp);
					Ok(())
				},
			),
		}
	}

	pub fn serialize(
		&self,
		entity: &hecs::EntityRef<'_>,
	) -> Result<Option<SerializedComponent>, AnyError> {
		(self.serialize)(entity)
	}

	pub fn deserialize(&self, bytes: Vec<u8>, builder: &mut hecs::EntityBuilder) -> VoidResult {
		(self.deserialize)(bytes, builder)
	}
}

struct FailedToSerialize(&'static str);
impl std::error::Error for FailedToSerialize {}
impl std::fmt::Debug for FailedToSerialize {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for FailedToSerialize {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "FailedToSerialize({})", self.0)
	}
}

struct FailedToDeserialize(&'static str);
impl std::error::Error for FailedToDeserialize {}
impl std::fmt::Debug for FailedToDeserialize {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for FailedToDeserialize {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "FailedToDeserialize({})", self.0)
	}
}
