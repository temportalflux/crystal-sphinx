mod data_file;
pub use data_file::*;

mod multi_hash_map;
pub use multi_hash_map::*;

pub fn get_named_arg(name: &str) -> Option<u16> {
	std::env::args().find_map(|arg| {
		let prefix = format!("-{}=", name);
		arg.strip_prefix(&prefix)
			.map(|s| s.parse::<u16>().ok())
			.flatten()
	})
}
