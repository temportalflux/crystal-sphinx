use crate::graphics::{
	model::{Model as ModelTrait, ModelBuilder},
	voxel::{
		atlas::Atlas,
		model::{self, Vertex},
	},
};
use engine::{
	graphics::{descriptor, sampler::Sampler},
	math::nalgebra::{Matrix4x2, Point2, Vector2, Vector4},
};
use std::sync::{Arc, Weak};

// Top-Left UV is -Horizontal & -Vertical
#[rustfmt::skip]
static TL_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 0.0, /*uNeg*/ 1.0,
	/*v*/ 0.0, /*uPos*/ 0.0,
	/*_*/ 0.0, /*vNeg*/ 1.0,
	/*_*/ 0.0, /*vPos*/ 0.0,
);
// Top-Right UV is +Horizontal & -Vertical
#[rustfmt::skip]
static TR_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 1.0, /*uNeg*/ 0.0,
	/*v*/ 0.0, /*uPos*/ 1.0,
	/*_*/ 0.0, /*vNeg*/ 1.0,
	/*_*/ 0.0, /*vPos*/ 0.0,
);
// Bottom-Left UV is -Horizontal & +Vertical
#[rustfmt::skip]
static BL_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 0.0, /*uNeg*/ 1.0,
	/*v*/ 1.0, /*uPos*/ 0.0,
	/*_*/ 0.0, /*vNeg*/ 0.0,
	/*_*/ 0.0, /*vPos*/ 1.0,
);
// Bottom-Right UV is +Horizontal & +Vertical
#[rustfmt::skip]
static BR_MATRIX: Matrix4x2<f32> = Matrix4x2::new(
	/*u*/ 1.0, /*uNeg*/ 0.0,
	/*v*/ 1.0, /*uPos*/ 1.0,
	/*_*/ 0.0, /*vNeg*/ 0.0,
	/*_*/ 0.0, /*vPos*/ 1.0,
);

#[derive(Default)]
pub struct Builder {
	is_opaque: bool,
	faces: Vec<model::FaceData>,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
	atlas: Option<(Arc<Atlas>, Arc<Sampler>, Weak<descriptor::Set>)>,
}

impl Builder {
	pub fn set_is_opaque(&mut self, is_opaque: bool) {
		self.is_opaque = is_opaque;
	}

	pub fn insert(&mut self, face_data: model::FaceData) {
		self.faces.push(face_data);
	}

	pub fn set_atlas(
		&mut self,
		atlas: Arc<Atlas>,
		sampler: Arc<Sampler>,
		descriptor_set: Weak<descriptor::Set>,
	) {
		self.atlas = Some((atlas, sampler, descriptor_set));
	}

	pub fn build(mut self) -> Model {
		let face_count = self.faces.len();
		// each face needs its own vectors because the texture data is embedded in each vertex
		self.vertices = Vec::with_capacity(face_count * 4); // 4 corners per face
		self.indices = Vec::with_capacity(face_count * 6); // two tris per face

		let entries = self.faces.drain(..).collect::<Vec<_>>();
		for face_data in entries.into_iter() {
			self.push_face(&face_data);
		}

		let (atlas, sampler, descriptor_set) = self.atlas.unwrap();
		Model {
			is_opaque: self.is_opaque,
			atlas,
			sampler,
			descriptor_set,
			vertices: self.vertices,
			indices: self.indices,
		}
	}
}

impl std::fmt::Debug for Builder {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.faces)
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
	fn push_face(&mut self, face_data: &model::FaceData) {
		let unified_flags: Vector4<f32> = face_data.flags.clone().into();

		let idx_tl = self.push_masked_vertex(&face_data, &TL_MATRIX, unified_flags);
		let idx_tr = self.push_masked_vertex(&face_data, &TR_MATRIX, unified_flags);
		let idx_br = self.push_masked_vertex(&face_data, &BR_MATRIX, unified_flags);
		let idx_bl = self.push_masked_vertex(&face_data, &BL_MATRIX, unified_flags);
		self.push_tri((idx_tl, idx_tr, idx_br));
		self.push_tri((idx_br, idx_bl, idx_tl));
	}

	fn push_masked_vertex(
		&mut self,
		face_data: &model::FaceData,
		mask_mat: &Matrix4x2<f32>,
		model_flags: Vector4<f32>,
	) -> u32 {
		let offset_mask: Vector4<f32> = mask_mat.column(1).into();
		let tex_coord_mask: Vector2<f32> = mask_mat.column(0).fixed_rows::<2>(0).into();

		let mut position = face_data.flags.face.model_offset_matrix() * offset_mask;
		position += face_data.flags.face.model_axis();

		let main_tex =
			face_data.main_tex.offset + face_data.main_tex.size.component_mul(&tex_coord_mask);
		let biome_color_mask = match face_data.biome_color_tex {
			Some(coord) => coord.offset + coord.size.component_mul(&tex_coord_mask),
			None => Point2::new(0.0, 0.0),
		};
		let tex_coord = Vector4::new(
			main_tex.x,
			main_tex.y,
			biome_color_mask.x,
			biome_color_mask.y,
		);

		self.push_vertex(Vertex {
			position: position.into(),
			tex_coord: tex_coord.into(),
			model_flags: model_flags.into(),
		})
	}
}

pub struct Model {
	is_opaque: bool,
	vertices: Vec<Vertex>,
	indices: Vec<u32>,
	#[allow(dead_code)]
	atlas: Arc<Atlas>,
	#[allow(dead_code)]
	sampler: Arc<Sampler>,
	descriptor_set: Weak<descriptor::Set>,
}

impl Model {
	pub fn builder() -> Builder {
		Builder::default()
	}

	pub fn index_count(&self) -> usize {
		self.indices.len()
	}

	pub fn descriptor_set(&self) -> Arc<descriptor::Set> {
		self.descriptor_set.upgrade().unwrap()
	}

	pub fn is_opaque(&self) -> bool {
		self.is_opaque
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
