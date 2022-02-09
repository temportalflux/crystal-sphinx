use crate::entity::component::Component;
use engine::utility::Result;
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
				fn_in_ref: Box::new(|entity_ref| -> bool { entity_ref.has::<T>() }),
				fn_in_builder: Box::new(|builder| -> bool { builder.has::<T>() }),
				fn_remove_from: Box::new(|world, entity| -> Result<()> {
					// Removed component will be dropped
					let _ = world.remove_one::<T>(entity)?;
					Ok(())
				}),
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
	fn_in_ref: Box<dyn Fn(&hecs::EntityRef) -> bool>,
	fn_in_builder: Box<dyn Fn(&hecs::EntityBuilder) -> bool>,
	fn_remove_from: Box<dyn Fn(&mut hecs::World, hecs::Entity) -> Result<()>>,
}
impl Registered {
	pub fn id(&self) -> &'static str {
		&self.id
	}

	pub fn display_name(&self) -> &'static str {
		&self.display_name
	}

	pub fn is_in_entity(&self, entity_ref: &hecs::EntityRef) -> bool {
		(self.fn_in_ref)(entity_ref)
	}

	pub fn is_in_builder(&self, builder: &hecs::EntityBuilder) -> bool {
		(self.fn_in_builder)(builder)
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

	pub fn get_ext_ok<T>(&self) -> Result<&T, Error>
	where
		T: 'static + ExtensionRegistration,
	{
		self.get_ext::<T>()
			.ok_or(Error::MissingExtension(T::extension_id(), self.id))
	}

	pub fn remove_from(&self, world: &mut hecs::World, entity: hecs::Entity) -> Result<()> {
		(self.fn_remove_from)(world, entity)
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

	pub fn find_ok(&self, id: &str) -> Result<&Registered, Error> {
		self.id_to_type
			.get(id)
			.map(|type_id| self.find(type_id))
			.flatten()
			.ok_or(Error::MissingRegistration(id.to_owned()))
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("No component registration found for component-type({0}).")]
	MissingRegistration(String),
	#[error("No registration extension \"{0}\" found for component-type({1}).")]
	MissingExtension(&'static str, &'static str),
}
