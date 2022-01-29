#[repr(u32)] // specifically a u32 so it fits in `socknet::Connection::close()`.
pub enum CloseCode {
	/// Error code for clients which failed authentication.
	/// Reason:
	/// Ø => token failed verification
	/// [0u8] => there was an error while processing the stream
	FailedAuthentication = 1,
}
