use nalgebra::{vector, Vector3};
use rapier3d::prelude::{
	BroadPhase, CCDSolver, Collider, ColliderHandle, ColliderSet, ImpulseJointSet,
	IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline,
	QueryPipeline, RigidBodySet,
};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct Physics(RwLock<State>);
impl Default for Physics {
	fn default() -> Self {
		Self(RwLock::new(State::default()))
	}
}
impl Physics {
	pub(super) fn write<'s>(&'s self) -> RwLockWriteGuard<'s, State> {
		self.0.write().unwrap()
	}

	pub fn read<'s>(&'s self) -> RwLockReadGuard<'s, State> {
		self.0.read().unwrap()
	}
}

pub struct State {
	pub(in crate::common::physics) gravity: Vector3<f32>,
	pub(in crate::common::physics) integration_parameters: IntegrationParameters,
	pub(in crate::common::physics) physics_pipeline: PhysicsPipeline,
	pub(in crate::common::physics) query_pipeline: QueryPipeline,
	pub(in crate::common::physics) islands: IslandManager,
	pub(in crate::common::physics) broad_phase: BroadPhase,
	pub(in crate::common::physics) narrow_phase: NarrowPhase,
	pub(in crate::common::physics) impulse_joints: ImpulseJointSet,
	pub(in crate::common::physics) multibody_joints: MultibodyJointSet,
	pub(in crate::common::physics) ccd_solver: CCDSolver,
	pub(in crate::common::physics) rigid_bodies: RigidBodySet,
	pub(in crate::common::physics) colliders: ColliderSet,
}
impl Default for State {
	fn default() -> Self {
		Self {
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
			rigid_bodies: RigidBodySet::new(),
			colliders: ColliderSet::new(),
		}
	}
}
impl State {
	pub fn collider(&self, handle: ColliderHandle) -> Option<&Collider> {
		self.colliders.get(handle)
	}
}
