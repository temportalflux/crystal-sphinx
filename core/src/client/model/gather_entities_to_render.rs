use crate::{
	client::model::{
		instance::{self, Instance},
		texture,
	},
	entity::{self, component},
};
use engine::{
	asset,
	Engine, EngineSystem,
};
use std::sync::{Arc, Mutex, RwLock, Weak};

use super::blender;

static LOG: &'static str = "subsystem:GatherEntitiesToRender";

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::physics::linear::Position,
	&'c component::Orientation,
	&'c blender::Component,
)>;

pub struct GatherEntitiesToRender {
	world: Weak<RwLock<entity::World>>,
	instance_buffer: Weak<RwLock<instance::Buffer>>,
	texture_cache: Weak<Mutex<texture::Cache>>,
}

impl GatherEntitiesToRender {
	pub fn create(
		world: Weak<RwLock<entity::World>>,
		instance_buffer: &Arc<RwLock<instance::Buffer>>,
		texture_cache: &Arc<Mutex<texture::Cache>>,
	) -> Arc<RwLock<Self>> {
		let arclocked = Arc::new(RwLock::new(Self {
			world,
			instance_buffer: Arc::downgrade(&instance_buffer),
			texture_cache: Arc::downgrade(&texture_cache),
		}));

		if let Ok(mut engine) = Engine::get().write() {
			engine.add_weak_system(Arc::downgrade(&arclocked));
		}

		arclocked
	}
}

impl EngineSystem for GatherEntitiesToRender {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!(LOG);

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let instance_buffer = match self.instance_buffer.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let texture_cache = self.texture_cache.upgrade();

		let mut instances = Vec::new();
		let mut entities = Vec::new();

		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (position, orientation, presentation)) in query_bundle.query(&world).iter() {
			let descriptor_id = DescriptorId {
				model_id: presentation.model().clone(),
				texture_id: presentation.texture().clone(),
			};
			instances.push(
				Instance::builder()
					.with_chunk(*position.chunk())
					.with_offset(*position.offset())
					.with_orientation(*orientation.orientation())
					.build(),
			);
			if let Some(arctex) = &texture_cache {
				if let Ok(mut cache) = arctex.lock() {
					cache.mark_required(&descriptor_id.texture_id);
				}
			}
			entities.push(descriptor_id);
		}

		if let Ok(mut buffer) = instance_buffer.write() {
			buffer.set_pending(entities, instances);
		}; // semi-colon here drops the `buffer` guard
	}
}

#[derive(Clone)]
pub struct DescriptorId {
	pub model_id: asset::Id,
	pub texture_id: asset::Id,
}
