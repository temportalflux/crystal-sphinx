use engine::graphics::{
	flags, pipeline,
	types::{Vec3, Vec4},
	vertex_object,
};

pub struct Mesh {
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
	/// The vertex groups of the mesh, which can be used by bones to apply animation transformations.
	/// Each element is a list of vertex weights which that group affects.
	vertex_groups: Vec<Vec<VertexWeight>>,
}

#[vertex_object]
#[derive(Debug, Default, Clone)]
pub struct Vertex {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub position: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub tex_coord: Vec4,
}

pub struct VertexWeight {
	/// The index of [`Mesh::vertices`] which the weight applies to.
	vertex_index: usize,
	/// How much of the transformation to apply to the vertex for this group.
	weight: f32,
}
