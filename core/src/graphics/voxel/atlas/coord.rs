use engine::math::nalgebra::{Point2, Vector2};

#[derive(Clone, Copy)]
pub struct AtlasTexCoord {
	pub(crate) offset: Point2<f32>,
	pub(crate) size: Vector2<f32>,
}

impl std::fmt::Debug for AtlasTexCoord {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}

impl std::fmt::Display for AtlasTexCoord {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"AtlasCoord(offset=<{}, {}> size=<{}, {}>)",
			self.offset.x, self.offset.y, self.size.x, self.size.y
		)
	}
}
