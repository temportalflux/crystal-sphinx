use crate::{
	client::model::instance::{self, Instance},
	entity::{self, component, ArcLockEntityWorld},
	graphics::voxel::camera,
};
use engine::{
	asset,
	math::nalgebra::{Point3, UnitQuaternion},
	Engine, EngineSystem,
};
use std::sync::{Arc, RwLock, Weak};

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
}

impl GatherEntitiesToRender {
	pub fn create(
		world: Weak<RwLock<entity::World>>,
		instance_buffer: &Arc<RwLock<instance::Buffer>>,
	) -> Arc<RwLock<Self>> {
		let arclocked = Arc::new(RwLock::new(Self {
			world,
			instance_buffer: Arc::downgrade(&instance_buffer),
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

		let mut instances = Vec::new();
		let mut entities = Vec::new();

		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (entity, (position, orientation, presentation)) in query_bundle.query(&world).iter() {
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
			entities.push(descriptor_id);
		}

		if let Ok(mut buffer) = instance_buffer.write() {
			buffer.set_pending(entities, instances);
		}; // semi-colon here drops the `buffer` guard
	}
}

pub struct DescriptorId {
	model_id: asset::Id,
	texture_id: asset::Id,
}
