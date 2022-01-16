use crate::entity::component::Component;
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
				extensions: HashMap::new(),
				fn_has: Box::new(|entity_ref| -> bool { entity_ref.has::<T>() }),
			},
			marker: Default::default(),
		}
	}
}
impl<T> Registration<T>
where
	T: Component,
{
	pub fn with_ext<TExt>(mut self, ext: TExt) -> Self
	where
		TExt: ExtensionRegistration + 'static,
	{
		self.item
			.extensions
			.insert(TExt::extension_id(), Box::new(ext));
		self
	}
}

pub struct Registered {
	id: &'static str,
	display_name: &'static str,
	extensions: HashMap<&'static str, Box<dyn std::any::Any>>,
	fn_has: Box<dyn Fn(&hecs::EntityRef) -> bool>,
}
impl Registered {
	pub fn id(&self) -> &'static str {
		&self.id
	}

	pub fn display_name(&self) -> &'static str {
		&self.display_name
	}

	pub fn is_in_entity(&self, entity_ref: &hecs::EntityRef) -> bool {
		(self.fn_has)(entity_ref)
	}

	pub fn has_ext<T>(&self) -> bool
	where
		T: 'static + ExtensionRegistration,
	{
		self.get_ext::<T>().is_some()
	}

	pub fn get_ext<T>(&self) -> Option<&T>
	where
		T: 'static + ExtensionRegistration,
	{
		self.extensions
			.get(T::extension_id())
			.map(|ext| ext.downcast_ref::<T>())
			.flatten()
	}
}

pub trait ExtensionRegistration {
	fn extension_id() -> &'static str
	where
		Self: Sized;
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
	pub fn register<T>(&mut self)
	where
		T: Component,
	{
		self.add(T::registration());
	}

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
