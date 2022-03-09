use engine::math::nalgebra::Vector3;
use std::ops::Range;

pub struct Mesh {
	/// The faces of the mesh.
	/// In theory this could contain n-gons, but in practice only triangles are allowed.
	polygons: Vec<Polygon>,
	/// The set of vertices without their UVs (this is a minimal set that will need to be expanded when a model is processed)
	vertices: Vec<Vertex>,
	/// The list of mappings for vertex uv coordinates as provided by polygon.
	/// This will likely contain duplicates and will need to be analyzed when a model is processed.
	vertex_uvs: Vec<WeightedGroup>,
}

pub struct WeightedGroup {
	group_index: usize,
	weight: f32,
}

pub struct Polygon {
	/// The normal vector of the face
	normal: Vector3<f32>,
	/// The indices of the vertices for this face.
	/// Since only triangles are allowed, this is always a ordered-list of 3.
	indices: Vec<usize>,
	/// The range of [`Mesh::vertex_uvs`] which this poly uses to map a vertex to its uv coordinates
	uv_range: Range<usize>,
}

pub struct Vertex {
	/// the position of the vertex
	position: Vector3<f32>,
	/// the normal of the vertex, not of the face it is used in
	normal: Vector3<f32>,
	/// the vertex groups this vertex is associated to and the weight per group
	groups: Vec<WeightedGroup>,
}

pub struct VertexUV {
	/// The index of the vertex in [`Mesh::vertices`]
	vertex_index: usize,
	/// The uv coordinate of the vertex
	uv: Vector2<f32>,
}
