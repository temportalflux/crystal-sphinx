use crate::entity::component::{binary, debug, Component};
use std::{any::TypeId, collections::HashMap};

pub struct Registration<T: Component> {
	item: Registered,
	marker: std::marker::PhantomData<T>,
}
impl<T> Default for Registration<T>
where
	T: Component,
{
	fn default() -> Self {
		Self {
			item: Registered {
				id: T::unique_id(),
				display_name: T::display_name(),
				binary: None,
				debug: None,
			},
			marker: Default::default(),
		}
	}
}
impl<T> Registration<T>
where
	T: Component,
{
	pub fn with_binary_serialization(mut self) -> Self
	where
		T: binary::Serializable,
	{
		self.item.binary = Some(binary::Registration::from::<T>());
		self
	}

	pub fn with_debug(mut self) -> Self
	where
		T: debug::EguiInformation,
	{
		self.item.debug = Some(debug::Registration::from::<T>());
		self
	}
}

pub struct Registered {
	id: &'static str,
	display_name: &'static str,
	binary: Option<binary::Registration>,
	debug: Option<debug::Registration>,
}
impl Registered {
	pub fn id(&self) -> &'static str {
		&self.id
	}
	pub fn display_name(&self) -> &'static str {
		&self.display_name
	}
	pub fn debug(&self) -> Option<&debug::Registration> {
		self.debug.as_ref()
	}
}

#[derive(Default)]
pub struct Registry {
	items: HashMap<TypeId, Registered>,
	id_to_type: HashMap<&'static str, TypeId>,
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
	pub fn add<T>(&mut self, registration: Registration<T>)
	where
		T: Component,
	{
		let id = TypeId::of::<T>();
		self.id_to_type.insert(registration.item.id, id);
		self.items.insert(id, registration.item);
	}

	pub fn get_type_id(&self, id: &str) -> Option<&TypeId> {
		self.id_to_type.get(id)
	}

	pub fn find(&self, type_id: &TypeId) -> Option<&Registered> {
		self.items.get(&type_id)
	}

	pub fn find_binary(&self, type_id: &TypeId) -> Option<&binary::Registration> {
		self.find(&type_id).map(|reg| reg.binary.as_ref()).flatten()
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

struct NotRegisteredAsBinarySerializable(String);
impl std::error::Error for NotRegisteredAsBinarySerializable {}
impl std::fmt::Debug for NotRegisteredAsBinarySerializable {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for NotRegisteredAsBinarySerializable {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "NotRegisteredAsBinarySerializable({})", self.0)
	}
}
