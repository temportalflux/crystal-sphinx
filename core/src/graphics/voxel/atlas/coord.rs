use engine::math::nalgebra::{Matrix2, Vector2};

pub struct AtlasTexCoord(Matrix2<f32>);

impl AtlasTexCoord {
	pub fn new(offset: Vector2<f32>, size: Vector2<f32>) -> Self {
		Self(Matrix2::from_columns(&[offset, size]))
	}

	pub fn offset(&self) -> Vector2<f32> {
		self.0.column(0).into()
	}

	pub fn size(&self) -> Vector2<f32> {
		self.0.column(1).into()
	}
}

impl std::fmt::Display for AtlasTexCoord {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"texture::Coord(offset=<{}, {}> size=<{}, {}>)",
			self.offset().x,
			self.offset().y,
			self.size().x,
			self.size().y
		)
	}
}
