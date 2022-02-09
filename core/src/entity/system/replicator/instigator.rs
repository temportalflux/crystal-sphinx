use crate::entity::component::{chunk::Relevancy, physics::linear::Position};
use engine::math::nalgebra::{Point3, Vector3};
use std::collections::HashSet;

pub struct UpdatedEntity {
	pub entity: hecs::Entity,
	pub old_chunk: Option<Point3<i64>>,
	pub new_chunk: Point3<i64>,
}

impl UpdatedEntity {
	pub fn acknowledged(entity: &hecs::Entity, position: &mut Position) -> Option<Self> {
		let old_chunk = position.prev_chunk().clone();
		let new_chunk = *position.chunk();
		position.acknowledge_chunk();
		if let Some(old_chunk) = &old_chunk {
			if new_chunk == *old_chunk {
				return None;
			}
		}
		Some(Self {
			entity: *entity,
			old_chunk,
			new_chunk,
		})
	}
}

#[derive(Clone, Copy)]
pub enum EntityOperation {
	/// Entity was spawned within relevancy range, or has entered relevancy range.
	/// Entities can enter range when relevancy-owning entities move, changing what chunks are relevant,
	/// or when an entity moves into range of relevancy-owning entities.
	Relevant,
	/// Entity was already relevant, but its data has changed.
	Update,
	/// Entity is no longer in relevancy range.
	Irrelevant,
	/// Entity was destroyed while in relevancy range.
	Destroyed,
}
