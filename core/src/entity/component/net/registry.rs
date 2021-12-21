use super::Replicated;
use engine::utility::{AnyError, VoidResult};
use serde::{Deserialize, Serialize};
use std::{any::TypeId, collections::HashMap};

#[allow(dead_code)]
struct Item {
	serialize: Box<dyn Fn(&hecs::EntityRef<'_>) -> Result<Option<SerializedComponent>, AnyError>>,
	deserialize: Box<dyn Fn(Vec<u8>, &mut hecs::EntityBuilder) -> VoidResult>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedEntity {
	pub entity: hecs::Entity,
	pub components: Vec<SerializedComponent>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializedComponent {
	id: String,
	data: Vec<u8>,
}

#[derive(Default)]
pub struct Registry {
	items: HashMap<&'static str, Item>,
	type_to_id: HashMap<TypeId, &'static str>,
}

impl Registry {
	fn get() -> &'static std::sync::RwLock<Self> {
		use engine::utility::singleton::*;
		static mut INSTANCE: Singleton<Registry> = Singleton::uninit();
		unsafe { INSTANCE.get_or_default() }
	}

	pub fn write() -> std::sync::RwLockWriteGuard<'static, Self> {
		Self::get().write().unwrap()
	}

	pub fn read() -> std::sync::RwLockReadGuard<'static, Self> {
		Self::get().read().unwrap()
	}
}

impl Registry {
	pub fn register<T>(&mut self)
	where
		T: hecs::Component + Replicated,
	{
		self.type_to_id.insert(TypeId::of::<T>(), T::unique_id());
		self.items.insert(
			T::unique_id(),
			Item {
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
			},
		);
	}

	pub(crate) fn serialize(
		&self,
		e: &hecs::EntityRef<'_>,
		id: TypeId,
	) -> Result<Option<SerializedComponent>, AnyError> {
		let id = match self.type_to_id.get(&id) {
			Some(id) => id,
			None => return Ok(None),
		};
		let item = self.items.get(id).ok_or(NoSuchId(id.to_string()))?;
		(item.serialize)(e)
	}

	pub(crate) fn deserialize(
		&self,
		serialized: SerializedComponent,
		builder: &mut hecs::EntityBuilder,
	) -> VoidResult {
		let item = self
			.items
			.get(serialized.id.as_str())
			.ok_or(NoSuchId(serialized.id))?;
		(item.deserialize)(serialized.data, builder)
	}

	pub(crate) fn serialize_entity(
		&self,
		entity_ref: hecs::EntityRef<'_>,
	) -> Result<SerializedEntity, AnyError> {
		let mut serialized_components = Vec::new();
		for type_id in entity_ref.component_types() {
			// None means NO-OP: the entity did not have a component of the given type
			if let Some(data) = self.serialize(&entity_ref, type_id)? {
				serialized_components.push(data);
			}
		}
		Ok(SerializedEntity {
			entity: entity_ref.entity(),
			components: serialized_components,
		})
	}
}

struct NoSuchId(String);
impl std::error::Error for NoSuchId {}
impl std::fmt::Debug for NoSuchId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for NoSuchId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "NoSuchId({})", self.0)
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
