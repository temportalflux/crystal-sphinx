use crate::{
	entity::{self, component, ArcLockEntityWorld},
	graphics::voxel::camera,
};
use engine::{math::nalgebra::Point3, EngineSystem};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::Position,
	&'c component::Orientation,
	&'c component::Camera,
)>;

pub struct UpdateCamera {
	world: Weak<RwLock<entity::World>>,
	camera: Arc<RwLock<camera::Camera>>,
}

impl UpdateCamera {
	pub fn new(world: &ArcLockEntityWorld, camera: Arc<RwLock<camera::Camera>>) -> Self {
		Self {
			world: Arc::downgrade(&world),
			camera,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for UpdateCamera {
	fn update(&mut self, _delta_time: std::time::Duration) {
		profiling::scope!("subsystem:update_camera");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		let mut result = self.camera.read().unwrap().clone();
		for (_entity, (position, orientation, camera)) in query_bundle.query(&world).iter() {
			// WARN: Casting i64 to f32 will result in data loss...
			// I'll find a way to address this on another day...
			let chunk = position.chunk();
			result.chunk_coordinate =
				Point3::new(chunk[0] as f32, chunk[1] as f32, chunk[2] as f32);

			result.position = *position.offset() + *camera.offset();
			result.orientation = **orientation;
			result.projection = *camera.projection();

			// TODO: support multiple camera components but only 1 active at a time
			break;
		}

		*self.camera.write().unwrap() = result;
	}
}
