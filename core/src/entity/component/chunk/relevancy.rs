use engine::math::nalgebra::{Point3, Vector3};
use std::collections::HashSet;

/// Component added on the server to indicate what chunks are relevant to a given entity.
/// Chunks which exist inside the radius are replicated, if the entity also has the
/// [`Owned By Connection`](crate::entity::component::OwnedByConnection) component.
#[derive(Clone)]
pub struct Relevancy {
	/// The radius of chunks around the [`current chunk coordinate`](crate::entity::component::physics::linear::Position::chunk).
	radius: usize,
	entity_radius: usize,
	/// The origin chunk of the last replication.
	/// Compared against the entity's [`current chunk coordinate`](crate::entity::component::physics::linear::Position::chunk)
	/// to determine if chunks need to be replicated.
	replicated_origin: Point3<i64>,
	/// Chunk coordinates which are relevant to the owner entity,
	/// but which have not yet been loaded by the server.
	/// These are repeatadley checked until they can be replicated
	/// and moved to [`replicated_chunks`](Relevancy::replicated_chunks).
	pending_chunks: Vec<Point3<i64>>,
	/// Chunk coordinates replicated to the owner of this component.
	/// This is updated before the replication packets are sent.
	replicated_chunks: HashSet<Point3<i64>>,
}

impl Default for Relevancy {
	fn default() -> Self {
		Self {
			radius: 0,
			entity_radius: 0,
			replicated_origin: Point3::new(0, 0, 0),
			pending_chunks: Vec::new(),
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

	pub fn with_entity_radius(mut self, radius: usize) -> Self {
		self.entity_radius = radius;
		self
	}

	pub fn entity_radius(&self) -> usize {
		self.entity_radius
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
		let centering_offset = Vector3::new(self.radius, self.radius, self.radius).cast::<i64>();
		for y in 0..diameter {
			for x in 0..diameter {
				for z in 0..diameter {
					coordinates.insert(origin + Vector3::new(x, y, z) - centering_offset);
				}
			}
		}
		coordinates
	}

	#[profiling::function]
	pub(crate) fn update_replicated_chunks(
		&mut self,
		origin: Point3<i64>,
		old: &HashSet<Point3<i64>>,
		new: &HashSet<Point3<i64>>,
	) {
		for coord in old.iter() {
			self.remove_pending(&coord);
			self.replicated_chunks.remove(&coord);
		}
		// sort according to the new origin
		self.replicated_origin = origin;
		self.pending_chunks.sort_by(|a, b| Self::cmp_coord_by_dist(a, b, &origin));
		for coord in new.iter() {
			self.insert_pending(*coord);
		}
	}

	fn cmp_coord_by_dist(a: &Point3<i64>, b: &Point3<i64>, origin: &Point3<i64>) -> std::cmp::Ordering {
		let a_dist = (a - origin).cast::<f32>().magnitude_squared();
		let b_dist = (b - origin).cast::<f32>().magnitude_squared();
		a_dist.partial_cmp(&b_dist).unwrap()
	}

	fn find_pending(&self, coord: &Point3<i64>) -> Result<usize, usize> {
		self.pending_chunks.binary_search_by(|a| Self::cmp_coord_by_dist(a, coord, &self.replicated_origin))
	}
	
	fn remove_pending(&mut self, coord: &Point3<i64>) {
		// If `find_pending` returns Ok, then the coordinate was found and can be removed.
		if let Ok(idx) = self.find_pending(&coord) {
			let _ = self.pending_chunks.remove(idx);
		}
	}

	pub fn insert_pending(&mut self, coord: Point3<i64>) {
		// If `find_pending` returns Err, then the coordinate was not found and,
		// and the resulting index is where it can be inserted to preserve sort order.
		if let Err(idx) = self.find_pending(&coord) {
			self.pending_chunks.insert(idx, coord);
		}
	}

	pub(crate) fn take_pending_chunks(&mut self, count: usize) -> HashSet<Point3<i64>> {
		let count = self.pending_chunks.len().min(count);
		self.pending_chunks.drain(..count).collect()
	}

	pub(crate) fn mark_as_replicated(&mut self, coord: Point3<i64>) {
		self.replicated_chunks.insert(coord);
	}
}
