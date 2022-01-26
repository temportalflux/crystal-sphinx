#[repr(u32)] // specifically a u32 so it fits in `socknet::Connection::close()`.
pub enum CloseCode {
	FailedAuthentication = 1,
}
