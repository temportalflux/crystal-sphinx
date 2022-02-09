use crate::{
	common::network::replication::entity::{Builder, Update},
	entity::{
		self, archetype,
		component::{self, binary::SerializedEntity},
	},
};
use engine::{
	network::socknet::{
		connection::{self, Connection},
		stream::{self, kind::recv::Ongoing},
	},
	utility::Result,
};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

pub type Context = stream::Context<Builder, Ongoing>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	recv: Ongoing,
	/// The Server->Client map of entity ids
	entity_map_s2c: HashMap</*server*/ hecs::Entity, /*client*/ hecs::Entity>,
}

impl From<Context> for Handler {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
			entity_map_s2c: HashMap::new(),
		}
	}
}

impl Handler {
	fn entity_world(&self) -> Result<Arc<RwLock<entity::World>>> {
		Ok(self
			.context
			.entity_world
			.upgrade()
			.ok_or(Error::InvalidEntityWorld)?)
	}
}

impl stream::handler::Receiver for Handler {
	type Builder = Builder;
	fn receive(mut self) {
		use connection::Active;
		use stream::Identifier;
		let log = format!(
			"client/{}[{}]",
			Builder::unique_id(),
			self.connection.remote_address()
		);
		engine::task::spawn(log.clone(), async move {
			use stream::kind::Read;
			log::info!(target: &log, "Stream opened");
			while let Ok(update) = self.recv.read::<Update>().await {
				if let Err(err) = self.process_update(&log, update) {
					log::error!(target: &log, "{:?}", err);
				}
			}
			Ok(())
		});
	}
}

impl Handler {
	fn process_update(&mut self, log: &str, update: Update) -> Result<()> {
		log::info!(target: &log, "Received update: {:?}", update);
		match update {
			Update::Relevant(serialized) => {
				self.spawn_entity(serialized)?;
			}
			Update::Update(serialized) => {
				let client_entity = match self.get_client_entity(&serialized.entity) {
					Some(entity) => entity,
					None => {
						log::warn!(target: &log, "Received entity replication Update({0}), but the client has not yet received the Relevant({0}) notification.", serialized.entity.id());
						return Ok(());
					}
				};
				self.update_entity(&log, client_entity, serialized)?;
			}
			Update::Irrelevant(server_entity) | Update::Destroyed(server_entity) => {
				self.despawn_entity(server_entity)?;
			}
		}
		Ok(())
	}

	fn get_client_entity(&self, server_entity: &hecs::Entity) -> Option<hecs::Entity> {
		self.entity_map_s2c.get(&server_entity).cloned()
	}

	fn is_builder_locally_owned(&self, builder: &hecs::EntityBuilder) -> bool {
		use crate::client::account;
		// This is only ever valid for players right now (only players have the OwnedByAccount component),
		// so until that condition changes, its safe to just apply the player client-only components to any owned entity.
		let local_account_id = account::Manager::read()
			.unwrap()
			.active_account()
			.map(|account| account.id());
		match (
			&local_account_id,
			builder.get::<component::OwnedByAccount>(),
		) {
			// If the account ids match, then this entity is the local player's avatar
			(Ok(local_id), Some(user)) => *user.id() == *local_id,
			_ => false,
		}
	}

	/// If the entity doesn't exist in the world, spawn it with the components.
	fn spawn_entity(&mut self, serialized: SerializedEntity) -> Result<()> {
		assert!(self.get_client_entity(&serialized.entity).is_none());
		let (server_entity, mut builder) = {
			let registry = component::Registry::read();
			serialized.into_builder(&registry)?
		};

		builder.add(component::network::Replicated::new_client(server_entity));

		// If this is first spawn and the entity is owned by the client, spawn the client-only components as well.
		if self.is_builder_locally_owned(&builder) {
			builder = archetype::player::Client::apply_to(builder);
		}

		let client_entity = {
			let arc = self.entity_world()?;
			let mut world = arc.write().unwrap();
			world.spawn(builder.build())
		};
		self.entity_map_s2c.insert(server_entity, client_entity);
		Ok(())
	}

	/// If the entity already exists in the world,
	/// update any existing components with the same types with the new data,
	/// spawn any missing components that were replicated,
	/// and destroy any components marked as replicated that are present locally but not replicated.
	fn update_entity(
		&self,
		log: &str,
		client_entity: hecs::Entity,
		serialized: SerializedEntity,
	) -> Result<()> {
		let _profiling_tag = format!(
			"server_entity={} client_entity={}",
			serialized.entity.id(),
			client_entity.id()
		);
		profiling::scope!("update_entity", &_profiling_tag);
		let registry = component::Registry::read();
		let (server_entity, builder) = serialized.into_builder(&registry)?;

		let arc_world = self.entity_world()?;
		let mut world = arc_world.write().unwrap();

		// Remove all components registered with the network extension (i.e. replicatable)
		// which are on the local entity but not the replicated builder
		// (i.e. they were previously created via a replication but no longer exist on the server).
		{
			profiling::scope!("remove-components", &_profiling_tag);
			let iter_to_remove = world
				.entity(client_entity)?
				.component_types()
				.filter_map(|type_id| registry.find(&type_id))
				.filter_map(|registered| {
					if registered
						.get_ext_ok::<component::network::Registration>()
						.is_ok()
					{
						if !registered.is_in_builder(&builder) {
							return Some(registered);
						}
					}
					None
				})
				.collect::<Vec<_>>();
			for registered in iter_to_remove {
				registered.remove_from(&mut world, client_entity)?;
			}
		}

		// Reference to the entity/components for the client entity in the world
		let entity_ref = world.entity(client_entity)?;

		let mut missing_components = hecs::EntityBuilder::new();
		let is_locally_owned = self.is_builder_locally_owned(&builder);

		// Iterate over all of the replicated components
		for type_id in builder.component_types() {
			// Get the registration for the component type
			let registered = registry.find(&type_id).unwrap();

			// Get the Replicatable registration extension for the component type
			let network_ext = registered.get_ext_ok::<component::network::Registration>()?;

			// Read the data from the replicated component into the existing entity.
			if !registered.is_in_entity(&entity_ref) {
				// cache the missing component to the builder for adding all missing components at once
				network_ext.clone_into_builder(&builder, &mut missing_components);
			} else {
				// read the data from the replicated component into the existing component
				network_ext.on_replication(&builder, &entity_ref, is_locally_owned);
			}
		}

		// Insert all of the components which were replicated but not yet on the entity (if any such exist).
		if missing_components.component_types().count() > 0 {
			world.insert(client_entity, missing_components.build())?;
		}

		Ok(())
	}

	fn despawn_entity(&mut self, server_entity: hecs::Entity) -> Result<()> {
		let client_match = self.entity_map_s2c.remove(&server_entity);
		assert!(client_match.is_some());
		if let Some(client_entity) = client_match {
			let arc = self.entity_world()?;
			let mut world = arc.write().unwrap();
			world.despawn(client_entity)?;
		}
		Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("Entity World is invalid")]
	InvalidEntityWorld,
}
