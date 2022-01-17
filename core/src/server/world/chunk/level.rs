/// The possible levels/states a chunk could be loaded as/in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
	/// Full game activity, including ticking of chunk and entities.
	/// Applied distance is configured by a provided [`Ticket`](super::Ticket).
	Ticking,
	/// Most game features are active, but nothing is updated on each tick.
	/// Applied distance is always 1.
	Active,
	/// Only some features are active. Blocks can be changed but nothing tickets.
	/// Applied distance is always 1.
	Minimal,
	/// The chunk is technically loaded and ready for activity, but no features are active.
	/// Only world generation occurs.
	/// Applied distance is always 1.
	Loaded,
}

impl Level {
	/// The list of levels which surround the current level.
	pub fn successive_levels(&self) -> Vec<Level> {
		match *self {
			Self::Ticking => vec![Self::Active, Self::Minimal, Self::Loaded],
			Self::Active => vec![Self::Minimal, Self::Loaded],
			Self::Minimal => vec![Self::Loaded],
			Self::Loaded => vec![],
		}
	}
}

/// A variation of the [`Level`](Level) enum which includes parameters for the levels, where applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterizedLevel {
	/// The provided member is the radius of the cuboid that should be loaded.
	/// All chunks in the cuboid-radius are loaded with the `Ticking` level.
	/// Beyond the cuboid, there is an additional layer/radius of 1 for each successive level (active, minimal, & loaded).
	///
	/// A radius of `0` means that:
	/// - 1 chunk is loaded as ticking
	/// - the 26 chunks (3^3 - 1^3) surrounding the Ticking chunk are loaded as Active
	/// - the next layer of 98 chunks (5^3 - 3^3) are loaded as Minimal
	/// - the next layer of 218 chunks (7^3 - 5^3) are loaded as Loaded
	/// In total, 343 chunks are loaded with a radius of 0.
	Ticking(/*radius*/ usize),
	Active,
	Minimal,
	Loaded,
}

impl std::fmt::Display for ParameterizedLevel {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Ticking(radius) => write!(f, "Ticking(radius={})", radius),
			Self::Active => write!(f, "Active"),
			Self::Minimal => write!(f, "Minimal"),
			Self::Loaded => write!(f, "Loaded"),
		}
	}
}

impl From<ParameterizedLevel> for Level {
	fn from(other: ParameterizedLevel) -> Self {
		match other {
			ParameterizedLevel::Ticking(_) => Self::Ticking,
			ParameterizedLevel::Active => Self::Active,
			ParameterizedLevel::Minimal => Self::Minimal,
			ParameterizedLevel::Loaded => Self::Loaded,
		}
	}
}

impl From<Level> for ParameterizedLevel {
	fn from(other: Level) -> Self {
		match other {
			Level::Active => Self::Active,
			Level::Minimal => Self::Minimal,
			Level::Loaded => Self::Loaded,
			_ => unimplemented!(),
		}
	}
}

impl From<(Level, usize)> for ParameterizedLevel {
	fn from(other: (Level, usize)) -> Self {
		match other {
			(Level::Ticking, radius) => Self::Ticking(radius),
			_ => unimplemented!(),
		}
	}
}
