use crate::server::world::{
	chunk::{Level, ParameterizedLevel},
	Database,
};
use anyhow::Result;
use engine::math::nalgebra::{Point3, Vector3};
use std::sync::Arc;

/// The channel through which chunk [tickets are sent](Ticket::submit).
pub(crate) type Sender = crossbeam_channel::Sender<std::sync::Weak<Ticket>>;
/// The channel through which chunk tickets are received by the [`chunk loading thread`](super::thread::start).
pub(crate) type Receiver = crossbeam_channel::Receiver<std::sync::Weak<Ticket>>;

/// A struct submitted at runtime to request that one or more chunks be loaded.
///
/// To change the coordinate or level of a ticket, drop the old ticket and submit a new one.
///
/// Largely inspired by <https://minecraft.fandom.com/wiki/Chunk#Java_Edition>.
pub struct Ticket {
	/// The coordinate of the chunk to be loaded.
	/// This is also the center of a cuboid with a radius determined by the `level` and `radius` properties.
	pub coordinate: Point3<i64>,
	/// The level the chunk should be loaded at.
	pub level: ParameterizedLevel,
}

impl std::fmt::Display for Ticket {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"Ticket(<{}, {}, {}> = {})",
			self.coordinate[0], self.coordinate[1], self.coordinate[2], self.level
		)
	}
}

impl Ticket {
	/// Wraps the ticket in a Arc-Mutex (Arctex), and then sends a weak clone through
	/// the chunk-loading channel to be processed by the loading thread.
	/// If the returned Arctex is dropped before the loading thread can process it, the request is canceled.
	/// If the arctex is dropped at any point in the future,
	/// the affected chunks will be unloaded if no other ticket references them.
	pub fn submit(self) -> Result<Arc<Ticket>> {
		let arctex = Arc::new(self);
		Database::send_chunk_ticket(&arctex)?;
		Ok(arctex)
	}

	pub(crate) fn coordinate_levels(&self) -> Vec<(Point3<i64>, Level)> {
		let mut points = Vec::new();

		let level: Level = self.level.into();
		points.push((self.coordinate, level));

		let mut prev_layer = 0;
		if let ParameterizedLevel::Ticking(radius) = self.level {
			for layer in 0..=radius {
				Self::visit_hollow_cube(layer, |point| {
					points.push((self.coordinate + point, Level::Ticking));
				});
			}
			prev_layer = radius;
		}

		for sublevel in level.successive_levels() {
			prev_layer += 1;
			Self::visit_hollow_cube(prev_layer, |point| {
				points.push((self.coordinate + point, sublevel));
			});
		}

		points
	}

	pub fn visit_hollow_cube<F>(radius: usize, mut callback: F)
	where
		F: FnMut(Vector3<i64>),
	{
		/* NOTE: This /could/ use the pattern:
			```
			for x in -extrema..=extrema {
				for z in -extrema..=extrema {
					for y in -extrema..=extrema {
						if x.abs() == extrema || y.abs() == extrema || z.abs() == extrema {
							callback(Vector3::new(x, y, z));
						}
					}
				}
			}
			```
		but that would mean if the function is called multiple times with increasing radii,
		we would be repeatadly filtering out values which are used in other layers.
		By writing out each extrema, we skip visiting values which are not in the extrema.
		Its more complicated and error prone, but also results in far fewer visitations.
		*/

		if radius == 0 {
			return;
		}

		// Visit corners (where extrema meet)
		let extrema = radius as i64;
		for x in [-extrema, extrema].iter() {
			for z in [-extrema, extrema].iter() {
				for y in [-extrema, extrema].iter() {
					callback(Vector3::new(*x, *y, *z));
				}
			}
		}

		// Visit edges without corners (only 1 axis changes per edge)
		let edge_radius = extrema - 1;
		for y in [-extrema, extrema].iter() {
			for x in [-extrema, extrema].iter() {
				for z in -edge_radius..=edge_radius {
					callback(Vector3::new(*x, *y, z));
				}
			}
			for z in [-extrema, extrema].iter() {
				for x in -edge_radius..=edge_radius {
					callback(Vector3::new(x, *y, *z));
				}
			}
		}
		for x in [-extrema, extrema].iter() {
			for z in [-extrema, extrema].iter() {
				for y in -edge_radius..=edge_radius {
					callback(Vector3::new(*x, y, *z));
				}
			}
		}

		// Visit faces without corner-edges (2 axis change)
		for x in [-extrema, extrema].iter() {
			for y in -edge_radius..=edge_radius {
				for z in -edge_radius..=edge_radius {
					callback(Vector3::new(*x, y, z));
				}
			}
		}
		for y in [-extrema, extrema].iter() {
			for x in -edge_radius..=edge_radius {
				for z in -edge_radius..=edge_radius {
					callback(Vector3::new(x, *y, z));
				}
			}
		}
		for z in [-extrema, extrema].iter() {
			for y in -edge_radius..=edge_radius {
				for x in -edge_radius..=edge_radius {
					callback(Vector3::new(x, y, *z));
				}
			}
		}
	}
}
