use engine::math::nalgebra::{Point3, Vector3};
use std::collections::HashSet;

#[derive(Clone)]
pub struct Relevancy {
	/// The radius of chunks around the [`current chunk coordinate`](super::Position::chunk)
	/// to replicate to the [`owner connection`](super::net::Owner::address).
	radius: usize,
	/// The origin chunk of the last replication.
	replicated_origin: Point3<i64>,
	/// Chunk coordinates which are relevant to the owner entity,
	/// but which have not yet been loaded by the server.
	/// These are repeatadley checked until they can be replicated
	/// and moved to [`replicated_chunks`](Relevancy::replicated_chunks).
	pending_chunks: HashSet<Point3<i64>>,
	/// Chunk coordinates replicated to the owner of this component.
	/// On the server, this is updated before the replication packets are sent.
	/// On the client, this is updated when the packets are received.
	replicated_chunks: HashSet<Point3<i64>>,
}

impl Default for Relevancy {
	fn default() -> Self {
		Self {
			radius: 0,
			replicated_origin: Point3::new(0, 0, 0),
			pending_chunks: HashSet::new(),
			replicated_chunks: HashSet::new(),
		}
	}
}

impl super::super::Component for Relevancy {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::chunk::Relevancy"
	}

	fn display_name() -> &'static str {
		"Chunk Relevancy"
	}
}

impl std::fmt::Display for Relevancy {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Relevancy(radius={})", self.radius)
	}
}

impl Relevancy {
	pub fn with_radius(mut self, radius: usize) -> Self {
		self.radius = radius;
		self
	}

	pub fn radius(&self) -> usize {
		self.radius
	}

	pub fn chunk(&self) -> &Point3<i64> {
		&self.replicated_origin
	}

	pub(crate) fn has_replicated_all(&self) -> bool {
		self.pending_chunks.len() == 0 && self.replicated_chunks.len() > 0
	}

	pub(crate) fn get_chunk_diff(
		&self,
		origin: &Point3<i64>,
	) -> (HashSet<Point3<i64>>, HashSet<Point3<i64>>) {
		let all_desired = self.relevant_chunk_list(&origin);
		// The chunks which the client doesnt have yet which need to be sent
		let new_chunks = all_desired.difference(&self.replicated_chunks);
		// The chunks the client had that are no longer relevant
		let old_chunks = self.replicated_chunks.difference(&all_desired);
		(new_chunks.cloned().collect(), old_chunks.cloned().collect())
	}

	fn relevant_chunk_list(&self, origin: &Point3<i64>) -> HashSet<Point3<i64>> {
		let diameter = 2 * self.radius + 1;
		let mut coordinates = HashSet::with_capacity(diameter.pow(3));
		let diameter = diameter as i64;
		for y in 0..diameter {
			for x in 0..diameter {
				for z in 0..diameter {
					coordinates.insert(origin + Vector3::new(x, y, z));
				}
			}
		}
		coordinates
	}

	pub(crate) fn update_replicated_chunks(
		&mut self,
		origin: Point3<i64>,
		old: &HashSet<Point3<i64>>,
		new: &HashSet<Point3<i64>>,
	) {
		self.replicated_origin = origin;
		self.pending_chunks.clear();
		for coord in old.iter() {
			self.replicated_chunks.remove(&coord);
		}
		for coord in new.iter() {
			self.pending_chunks.insert(*coord);
		}
	}

	pub(crate) fn take_pending_chunks(&mut self) -> HashSet<Point3<i64>> {
		self.pending_chunks.drain().collect()
	}

	pub(crate) fn mark_as_pending(&mut self, coord: Point3<i64>) {
		self.pending_chunks.insert(coord);
	}

	pub(crate) fn mark_as_replicated(&mut self, coord: Point3<i64>) {
		self.replicated_chunks.insert(coord);
	}

	pub(crate) fn update_with_replicated(
		&mut self,
		origin: Point3<i64>,
		old: &HashSet<Point3<i64>>,
		new: &HashSet<Point3<i64>>,
	) {
		self.replicated_origin = origin;
		for coord in old.iter() {
			self.replicated_chunks.remove(&coord);
		}
		for coord in new.iter() {
			self.mark_as_replicated(*coord);
		}
	}
}
