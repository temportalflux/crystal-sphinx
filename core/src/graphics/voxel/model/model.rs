use crate::graphics::{
	model::{Model as ModelTrait, ModelBuilder},
	voxel::{
		atlas::{Atlas, AtlasTexCoord},
		model::Vertex,
		Face, VertexFlags,
	},
};
use engine::math::nalgebra::{Matrix4x2, Vector2, Vector4};
use std::{collections::HashMap, sync::Arc};

#[rustfmt::skip]
static TL_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 0.0, /*uNeg*/ 1.0,
	/*v*/ 0.0, /*uPos*/ 0.0,
	/*_*/ 0.0, /*vNeg*/ 1.0,
	/*_*/ 0.0, /*vPos*/ 0.0,
);
#[rustfmt::skip]
static TR_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 1.0, /*uNeg*/ 0.0,
	/*v*/ 0.0, /*uPos*/ 1.0,
	/*_*/ 0.0, /*vNeg*/ 1.0,
	/*_*/ 0.0, /*vPos*/ 0.0,
);
#[rustfmt::skip]
static BL_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 0.0, /*uNeg*/ 1.0,
	/*v*/ 1.0, /*uPos*/ 0.0,
	/*_*/ 0.0, /*vNeg*/ 0.0,
	/*_*/ 0.0, /*vPos*/ 1.0,
);
#[rustfmt::skip]
static BR_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 1.0, /*uNeg*/ 0.0,
	/*v*/ 1.0, /*uPos*/ 1.0,
	/*_*/ 0.0, /*vNeg*/ 0.0,
	/*_*/ 0.0, /*vPos*/ 1.0,
);

#[derive(Default)]
pub struct Builder {
	faces: HashMap<Face, AtlasTexCoord>,
	atlas: Option<Arc<Atlas>>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
}

impl Builder {
	pub fn insert(&mut self, face: Face, tex_coord: AtlasTexCoord) {
		self.faces.insert(face, tex_coord);
	}

	pub fn set_atlas(&mut self, atlas: Arc<Atlas>) {
		self.atlas = Some(atlas);
	}

	pub fn build(mut self) -> Model {
		let face_count = self.faces.len();
		// each face needs its own vectors because the texture data is embedded in each vertex
		self.vertices = Vec::with_capacity(face_count * 4); // 4 corners per face
		self.indices = Vec::with_capacity(face_count * 6); // two tris per face
		let coords = self.faces.drain().collect::<Vec<_>>();
		for (face, tex_coord) in coords.into_iter() {
			self.push_face(face, &tex_coord);
		}
		Model {
			atlas: self.atlas.unwrap(),
			vertices: self.vertices,
			indices: self.indices,
		}
	}
}

impl std::fmt::Debug for Builder {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let face_string_list = self
			.faces
			.iter()
			.map(|(face, tex_coord)| {
				let tex_offset = tex_coord.offset;
				let tex_size = tex_coord.size;
				format!(
					"{} => (offset:<{}, {}>, size:<{}, {}>)",
					face, tex_offset[0], tex_offset[1], tex_size[0], tex_size[1]
				)
			})
			.collect::<Vec<_>>();
		write!(f, "[{}]", face_string_list.join(", "))
	}
}

impl ModelBuilder for Builder {
	type Vertex = Vertex;
	type Index = u32;

	fn vertices_mut(&mut self) -> &mut Vec<Self::Vertex> {
		&mut self.vertices
	}

	fn indices_mut(&mut self) -> &mut Vec<Self::Index> {
		&mut self.indices
	}

	fn get_next_index(&self) -> Self::Index {
		self.vertices.len() as u32
	}
}

impl Builder {
	fn push_face(&mut self, face: Face, tex_coord: &AtlasTexCoord) {
		let flags: Vector4<f32> = VertexFlags { face }.into();

		let idx_tl = self.push_masked_vertex(face, &tex_coord, &TL_MATRIX, flags);
		let idx_tr = self.push_masked_vertex(face, &tex_coord, &TR_MATRIX, flags);
		let idx_bl = self.push_masked_vertex(face, &tex_coord, &BL_MATRIX, flags);
		let idx_br = self.push_masked_vertex(face, &tex_coord, &BR_MATRIX, flags);
		self.push_tri((idx_tl, idx_tr, idx_br));
		self.push_tri((idx_br, idx_bl, idx_tl));
	}

	fn push_masked_vertex(
		&mut self,
		face: Face,
		tex_coord: &AtlasTexCoord,
		mask_mat: &Matrix4x2<f32>,
		model_flags: Vector4<f32>,
	) -> u32 {
		let offset_mask: Vector4<f32> = mask_mat.column(1).into();
		let tex_coord_mask: Vector2<f32> = mask_mat.column(0).fixed_rows::<2>(0).into();

		let position = (face.model_offset_matrix() * offset_mask) + face.model_axis();
		let tex_coord = tex_coord.offset + tex_coord.size.component_mul(&tex_coord_mask);

		self.push_vertex(Vertex {
			position: position.into(),
			tex_coord: tex_coord.into(),
			model_flags: model_flags.into(),
		})
	}
}

pub struct Model {
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
	atlas: Arc<Atlas>,
	// TODO: texture descriptor sets
}

impl Model {
	pub fn builder() -> Builder {
		Builder::default()
	}
}

impl ModelTrait for Model {
	type Vertex = Vertex;
	type Index = u32;

	fn vertices(&self) -> &Vec<Self::Vertex> {
		&self.vertices
	}

	fn indices(&self) -> &Vec<Self::Index> {
		&self.indices
	}
}
