use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::entity::component::Registry;

#[derive(Serialize, Deserialize, Clone)]
pub struct SerializedEntity {
	pub entity: hecs::Entity,
	pub components: Vec<SerializedComponent>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializedComponent {
	pub(crate) id: String,
	pub(crate) data: Vec<u8>,
}

impl SerializedEntity {
	pub fn into_builder(self, registry: &Registry) -> Result<(hecs::Entity, hecs::EntityBuilder)> {
		profiling::scope!(
			"deserialize-entity",
			&format!("entity={}", self.entity.id())
		);
		let mut builder = hecs::EntityBuilder::default();
		for comp_data in self.components.into_iter() {
			profiling::scope!(
				"deserialize-component",
				&format!("entity={} component={}", self.entity.id(), comp_data.id)
			);
			let registered = registry.find_ok(&comp_data.id)?;
			let binary_registration = registered.get_ext_ok::<Registration>()?;
			binary_registration.deserialize(comp_data.data, &mut builder)?;
		}
		Ok((self.entity, builder))
	}
}

/// Trait implemented by components to provide functionality for serializing to and deserializing from binary data.
pub trait Serializable: super::Component {
	fn serialize(&self) -> Result<Vec<u8>>
	where
		Self: Sized;
	fn deserialize(bytes: Vec<u8>) -> Result<Self>
	where
		Self: Sized;
}

pub fn serialize<T>(comp: &T) -> Result<Vec<u8>>
where
	T: serde::Serialize + Sized,
{
	Ok(bincode::serialize(&comp)?)
}

pub fn deserialize<'a, T>(bytes: &'a Vec<u8>) -> Result<T>
where
	T: serde::Deserialize<'a>,
{
	Ok(bincode::deserialize(&bytes)?)
}

pub struct Registration {
	serialize: Box<dyn Fn(&hecs::EntityRef<'_>) -> Result<Option<SerializedComponent>>>,
	deserialize: Box<dyn Fn(Vec<u8>, &mut hecs::EntityBuilder) -> Result<()>>,
}
impl super::ExtensionRegistration for Registration {
	fn extension_id() -> &'static str
	where
		Self: Sized,
	{
		"binary-serializable"
	}
}
impl Registration {
	pub(crate) fn from<T>() -> Self
	where
		T: super::Component + Serializable,
	{
		Self {
			serialize: Box::new(
				|e: &hecs::EntityRef<'_>| -> Result<Option<SerializedComponent>> {
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
				|bytes: Vec<u8>, builder: &mut hecs::EntityBuilder| -> Result<()> {
					profiling::scope!("deserialize-component", T::unique_id());
					let comp =
						T::deserialize(bytes).map_err(|_| FailedToDeserialize(T::unique_id()))?;
					builder.add(comp);
					Ok(())
				},
			),
		}
	}

	pub fn serialize(&self, entity: &hecs::EntityRef<'_>) -> Result<Option<SerializedComponent>> {
		(self.serialize)(entity)
	}

	pub fn deserialize(&self, bytes: Vec<u8>, builder: &mut hecs::EntityBuilder) -> Result<()> {
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
