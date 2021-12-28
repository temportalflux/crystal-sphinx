use engine::math::nalgebra::{Point2, Vector2};

#[derive(Clone, Copy)]
pub struct AtlasTexCoord {
	pub(crate) offset: Point2<f32>,
	pub(crate) size: Vector2<f32>,
}

impl std::fmt::Display for AtlasTexCoord {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"texture::Coord(offset=<{}, {}> size=<{}, {}>)",
			self.offset.x, self.offset.y, self.size.x, self.size.y
		)
	}
}
