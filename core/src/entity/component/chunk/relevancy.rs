/// Component added on the server to indicate what chunks are relevant to a given entity.
/// Chunks which exist inside the radius are replicated, if the entity also has the
/// [`Owned By Connection`](crate::entity::component::OwnedByConnection) component.
#[derive(Clone)]
pub struct Relevancy {
	/// The radius of chunks around the [`current chunk coordinate`](crate::entity::component::physics::linear::Position::chunk).
	radius: u64,
	entity_radius: u64,
}

impl Default for Relevancy {
	fn default() -> Self {
		Self {
			radius: 0,
			entity_radius: 0,
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
	pub fn with_radius(mut self, radius: u64) -> Self {
		self.radius = radius;
		self
	}

	pub fn radius(&self) -> u64 {
		self.radius
	}

	pub fn with_entity_radius(mut self, radius: u64) -> Self {
		self.entity_radius = radius;
		self
	}

	pub fn entity_radius(&self) -> u64 {
		self.entity_radius
	}
}
