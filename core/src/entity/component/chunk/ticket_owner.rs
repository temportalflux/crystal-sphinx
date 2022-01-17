use crate::server::world::chunk;
use engine::math::nalgebra::Point3;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ActiveTicket {
	coordinate: Point3<i64>,
	#[allow(dead_code)]
	handle: Arc<chunk::Ticket>,
}

#[derive(Clone, Default)]
pub struct TicketOwner {
	/// The radius of chunks around the [`current chunk coordinate`](super::Position::chunk)
	/// to load on the server.
	server_load_radius: usize,

	/// The ticket on the server that keeps chunks around the entity loaded.
	current_ticket: Option<ActiveTicket>,
}

impl super::super::Component for TicketOwner {
	fn unique_id() -> &'static str {
		"crystal_sphinx::entity::component::chunk::TicketOwner"
	}

	fn display_name() -> &'static str {
		"Chunk Ticket Owner"
	}
}

impl std::fmt::Display for TicketOwner {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"TicketOwner({})",
			match &self.current_ticket {
				Some(active) => format!(
					"<{}, {}, {}>",
					active.coordinate[0], active.coordinate[1], active.coordinate[2]
				),
				None => "None".to_owned(),
			}
		)
	}
}

impl TicketOwner {
	pub fn with_load_radius(mut self, radius: usize) -> Self {
		self.server_load_radius = radius;
		self
	}

	pub(crate) fn ticket_coordinate(&self) -> Option<Point3<i64>> {
		self.current_ticket.as_ref().map(|active| active.coordinate)
	}

	pub(crate) fn submit_ticket(&mut self, coordinate: Point3<i64>) {
		let scope_tag = format!("<{}, {}, {}>", coordinate[0], coordinate[1], coordinate[2]);
		profiling::scope!("submit_ticket", scope_tag.as_str());
		self.current_ticket = None;
		let ticket = chunk::Ticket {
			coordinate,
			level: (chunk::Level::Ticking, self.server_load_radius).into(),
		};
		if let Ok(handle) = ticket.submit() {
			self.current_ticket = Some(ActiveTicket { coordinate, handle })
		}
	}
}
