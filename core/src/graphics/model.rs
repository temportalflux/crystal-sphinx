pub trait Model {
	type Vertex;
	type Index;

	fn vertices(&self) -> &Vec<Self::Vertex>;
	fn vertices_mut(&mut self) -> &mut Vec<Self::Vertex>;

	fn indices(&self) -> &Vec<Self::Index>;
	fn indices_mut(&mut self) -> &mut Vec<Self::Index>;
	fn get_next_index(&self) -> Self::Index;

	fn push_vertex(&mut self, value: Self::Vertex) -> Self::Index {
		let index = self.get_next_index();
		self.vertices_mut().push(value);
		index
	}

	fn push_index(&mut self, value: Self::Index) {
		self.indices_mut().push(value);
	}

	fn push_tri(&mut self, tri: (Self::Index, Self::Index, Self::Index)) {
		self.push_index(tri.0);
		self.push_index(tri.1);
		self.push_index(tri.2);
	}
}
