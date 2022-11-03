//! Inspired by https://github.com/leetvr/hotham/blob/d355bb1c996682900eab64d7afb4c8f87a7d48c9/hotham/src/systems/physics.rs

use crate::entity::{self, ArcLockEntityWorld};
use engine::EngineSystem;
use nalgebra::{vector, Vector3};
use rand::Rng;
use rapier3d::prelude::{
	BroadPhase, CCDSolver, ColliderSet, ImpulseJointSet, IntegrationParameters,
	IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline,
	RigidBodySet,
};
use std::{
	sync::{Arc, RwLock, Weak},
	time::Duration,
};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c mut entity::component::physics::linear::Position,
	&'c entity::component::physics::linear::Velocity,
)>;

pub mod component;
mod state;
pub use state::*;
mod system;
pub use system::*;

pub struct SimplePhysics {
	world: Weak<RwLock<entity::World>>,
}

impl SimplePhysics {
	pub fn new(world: &ArcLockEntityWorld) -> Self {
		Self {
			world: Arc::downgrade(&world),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for SimplePhysics {
	fn update(&mut self, delta_time: Duration, _: bool) {
		profiling::scope!("subsystem:simple-physics");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (position, velocity)) in query_bundle.query_mut(&mut world) {
			let velocity_vec = **velocity;
			if velocity_vec.magnitude_squared() > 0.0 {
				*position += velocity_vec * delta_time.as_secs_f32();
			}
		}
	}
}
