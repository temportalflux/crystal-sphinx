use engine::{
	asset,
	graphics::Texture,
	math::nalgebra::{Point2, Vector2},
};
use std::collections::HashMap;

struct Entry {
	coord: Point2<usize>,
	size: Vector2<usize>,
	uv: Point2<f32>,
	binary: Vec<u8>,
}

pub struct Atlas {
	size: Vector2<usize>,
	cell_size: Vector2<usize>,

	next_coord: Point2<usize>,

	entries: HashMap<asset::Id, Entry>,
	save_entries: bool,
}

impl Default for Atlas {
	fn default() -> Self {
		Self {
			size: Vector2::new(2048, 2048),
			cell_size: Vector2::new(16, 16),
			next_coord: Point2::new(0, 0),
			entries: HashMap::new(),
			save_entries: true,
		}
	}
}

impl std::fmt::Display for Atlas {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let width_remaining_in_row = self.size.x - self.next_coord.x;
		let remaining_in_row = width_remaining_in_row / self.cell_size.x;
		let next_row_y = self.next_coord.y + self.cell_size.y;
		let height_remaining = self.size.y - next_row_y;
		let remaining_rows = height_remaining / self.cell_size.y;
		let cells_per_row = self.size.x / self.cell_size.x;
		let cells_remaining = remaining_in_row + remaining_rows * cells_per_row;
		write!(
			f,
			"Atlas(cells_remaining={}, stitched=[{}])",
			cells_remaining,
			self.entries
				.iter()
				.map(|(id, Entry { coord, .. })| format!("{}=<{}, {}>", id, coord.x, coord.y))
				.collect::<Vec<_>>()
				.join(", ")
		)
	}
}

impl Atlas {
	fn create_stub(&self) -> Self {
		Self {
			next_coord: self.next_coord,
			size: self.size,
			cell_size: self.cell_size,
			save_entries: false,
			entries: HashMap::new(),
		}
	}

	/// Returns true if the atlas either already
	/// contains or can fit all of the provided textures.
	pub fn contains_or_fits_all(&self, textures: &HashMap<&asset::Id, &Box<Texture>>) -> bool {
		let texture_to_fit = textures.iter().filter_map(|(id, texture)| {
			match self.entries.contains_key(&id) {
				// we dont need to check if it fits if its already stitched
				true => None,
				false => Some((*id, *texture)),
			}
		});
		let mut stub = self.create_stub();
		for (id, texture) in texture_to_fit {
			if stub.insert(&id, &texture).is_ok() {
				return false;
			}
		}
		true
	}

	pub fn insert_all(
		&mut self,
		textures: &HashMap<&asset::Id, &Box<Texture>>,
	) -> std::result::Result<(), InsertionError> {
		for (id, texture) in textures.iter() {
			if !self.entries.contains_key(&id) {
				let _ = self.insert(&id, &texture)?;
			}
		}
		Ok(())
	}

	pub fn insert(
		&mut self,
		id: &asset::Id,
		texture: &Texture,
	) -> std::result::Result<Point2<usize>, InsertionError> {
		use InsertionError::*;
		let size = texture.size();
		// All items must be the same size.
		if *size != self.cell_size {
			return Err(DoesNotMatchAtlasCellSize(id.clone(), *size, self.cell_size));
		}
		// Cannot fit any more if the next cell is outside of the atlas.
		if self.next_coord.x == self.size.x && self.next_coord.y == self.size.y {
			return Err(OutOfSpace(id.clone()));
		}

		// Allocate the coordinate and texture data
		let coord = self.next_coord.clone();
		// But don't save entries if this is a stub.
		if self.save_entries {
			self.entries.insert(
				id.clone(),
				Entry {
					coord: coord.clone(),
					size: texture.size().clone(),
					uv: Point2::new(
						coord.x as f32 / self.size.x as f32,
						coord.y as f32 / self.size.y as f32,
					),
					binary: texture.binary().clone(),
				},
			);
		}

		// It fits, lets bump the next coord to the next column.
		self.next_coord.x += size.x;
		// If the next column is outside the size,
		// jump to the first column of the next row.
		if self.next_coord.x == self.size.x {
			self.next_coord.x = 0;
			self.next_coord.y += size.y;
		}
		Ok(coord)
	}

	pub fn get(&self, id: &asset::Id) -> Option<&Point2<f32>> {
		self.entries.get(&id).map(|entry| &entry.uv)
	}

	pub fn as_binary(&self) -> Vec<u8> {
		// 4 per pixel for each RGBA channel
		let size = self.size.x * self.size.y * 4;
		let mut binary = Vec::with_capacity(size);
		binary.resize(size, 0);
		for (_id, entry) in self.entries.iter() {
			for y in 0..entry.coord.y {
				for x in 0..entry.coord.x {
					for channel in 0..4 {
						let src = Vector2::new(x, y);
						let dst = entry.coord + src;
						let src_pixel = (src.y * entry.size.x * 4) + (src.x * 4) + channel;
						let dst_pixel = (dst.y * self.size.x * 4) + (dst.x * 4) + channel;
						binary[dst_pixel] = entry.binary[src_pixel];
					}
				}
			}
		}
		binary
	}
}

pub enum InsertionError {
	DoesNotMatchAtlasCellSize(asset::Id, Vector2<usize>, Vector2<usize>),
	OutOfSpace(asset::Id),
}
impl std::error::Error for InsertionError {}
impl std::fmt::Debug for InsertionError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		<Self as std::fmt::Display>::fmt(&self, f)
	}
}
impl std::fmt::Display for InsertionError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::DoesNotMatchAtlasCellSize(id, tex_size, cell_size) => write!(f, "Failed to insert {}, texture size <{}, {}> does not match expected size of each cell <{}, {}>.", 
				id, tex_size.x, tex_size.y, cell_size.x, cell_size.y
				)
			,
			Self::OutOfSpace(id) => write!(f, "Failed to insert {}, atlas is out of space.", id),
		}
	}
}
