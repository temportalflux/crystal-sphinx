//! Inspired by https://github.com/leetvr/hotham/blob/d355bb1c996682900eab64d7afb4c8f87a7d48c9/hotham/src/systems/physics.rs

use crate::entity::{self, ArcLockEntityWorld};
use engine::EngineSystem;
use nalgebra::{vector, Vector3};
use rand::Rng;
use rapier3d::prelude::{
	BroadPhase, CCDSolver, ColliderBuilder, ColliderSet, ImpulseJointSet, IntegrationParameters,
	IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline,
	RigidBodyBuilder, RigidBodySet,
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
mod phase;

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

pub struct Context {
	// ----- System Configuration -----
	gravity: Vector3<f32>,
	integration_parameters: IntegrationParameters,
	physics_pipeline: PhysicsPipeline,
	query_pipeline: QueryPipeline,
	islands: IslandManager,
	broad_phase: BroadPhase,
	narrow_phase: NarrowPhase,
	impulse_joints: ImpulseJointSet,
	multibody_joints: MultibodyJointSet,
	ccd_solver: CCDSolver,
	// ----- Object Data -----
	rigid_bodies: RigidBodySet,
	colliders: Arc<RwLock<ColliderSet>>,
}

pub struct Physics {
	world: Weak<RwLock<entity::World>>,
	context: Context,
	simulation: phase::StepSimulation,
}

impl Physics {
	pub fn new(world: &Arc<RwLock<entity::World>>) -> Self {
		Self {
			world: Arc::downgrade(world),
			context: Context {
				// ----- System Configuration -----
				gravity: vector![0.0, -9.81, 0.0],
				integration_parameters: IntegrationParameters::default(),
				physics_pipeline: PhysicsPipeline::new(),
				query_pipeline: QueryPipeline::new(),
				islands: IslandManager::new(),
				broad_phase: BroadPhase::new(),
				narrow_phase: NarrowPhase::new(),
				impulse_joints: ImpulseJointSet::new(),
				multibody_joints: MultibodyJointSet::new(),
				ccd_solver: CCDSolver::new(),
				// ----- Object Data -----
				rigid_bodies: RigidBodySet::new(),
				colliders: Arc::new(RwLock::new(ColliderSet::new())),
			},
			simulation: phase::StepSimulation {
				duration_since_update: Duration::from_millis(0),
			},
		}
		.init_demo()
	}

	fn init_demo(mut self) -> Self {
		{
			let mut colliders = self.context.colliders.write().unwrap();

			colliders.insert(
				ColliderBuilder::cuboid(8.0, 0.5, 8.0)
					.translation(vector![8.0, 6.0, 8.0])
					.build(),
			);

			let mut rng = rand::thread_rng();
			let balls = vec![
				(vector![5.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.1),
				(vector![11.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.3),
				(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.5),
				(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 5.0], 0.7),
				(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 11.0], 0.9),
			];
			for (position, bounciness) in balls.into_iter() {
				let ball_rb = RigidBodyBuilder::dynamic()
					.translation(position)
					.ccd_enabled(true)
					.build();
				let ball_col = ColliderBuilder::ball(0.5).restitution(bounciness).build();
				let ball_handle = self.context.rigid_bodies.insert(ball_rb);
				colliders.insert_with_parent(ball_col, ball_handle, &mut self.context.rigid_bodies);
			}

			//let cyl_col = ColliderBuilder::cylinder(0.85, 0.4)
			//	.translation(vector![5.0, 10.0, 5.0])
			//	.build();
			//colliders.insert(cyl_col);
		}
		self
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn colliders(&self) -> &Arc<RwLock<ColliderSet>> {
		&self.context.colliders
	}
}

impl EngineSystem for Physics {
	fn update(&mut self, delta_time: Duration, _: bool) {
		profiling::scope!("subsystem:physics");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = {
			profiling::scope!("lock-world");
			arc_world.write().unwrap()
		};

		phase::AddPhysicsObjects::execute(&mut self.context, &mut world);
		phase::CopyComponentsToPhysics::execute(&mut self.context, &mut world);
		self.simulation.execute(&mut self.context, delta_time);
		phase::CopyPhysicsToComponents::execute(&mut self.context, &mut world);
	}
}