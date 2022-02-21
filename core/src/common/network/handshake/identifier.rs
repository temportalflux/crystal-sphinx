use crate::common::network::handshake::{client, server};
use socknet::{self, stream};
use std::sync::Arc;

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The identifier struct for connecting to a server (`handshake`).
///
/// Client-Initiated stream which handles the authentication protocol.
/// While clients are technically connected when the stream is initiated,
/// they don't really count as valid clients until the stream is concluded.
///
/// [Edit Diagram](https://mermaid.live/edit/#pako:eNp9VE1v2zAM_SuEL7t0f8CHAoVTbNnQbpiLnXxhJDoVIlOePgIERf_7KEtelzQofLBlvke-R9J-aZTT1LRNoD-JWNHG4N7jNPAQMUXHadqRz6cZfTTKzMgROsAAnWMmFUlDZw1xvAT1GdSTP17h31mbo_n2Iz6TrylCBnafb2_7Fr4i6_CMB4KtlpAZTclTw4XwKcDPtLNGwXc65eijiwROSkLGoFXJorxRKzqxEZ-w3bwD_yLUgEq5JAI1RnyH-I3W6JxtLiUPpWQVtDFhtniCR5zoHTXNC1FXDF_D7InJZ5QX526C6A7EGdVLga6FuySdkkYojMYxPK3htyQCCmbPb8wqrZeXMqYrjOyKvBlPC1EwCxVwj4ZDvDBadQg7E46Zt4oJEWMK_4HuWUMfPdVNshHMCDOGIDXwzEiOX0jq8UiQghzWOVwOVhLLMSeJJxidrwMuUNm0undiB7as3GR4X2IX7XqSXgdT-ynQL3Uw-apOyqLBN2ekPyWG2dy_AssChzOW7PVV4ln1BdTHtCsx4gVENlDu1YjGJl_FrFLKF5fVdtaFNesHjh5kjg_EaclcCjQ3zUR-QqPlq38ZGGBoxIH4blp51OgPQzPwq-DK1t5rE51v2hFF2k2T_wr9iVXTRp9oBdXfRkW9_gX2CJCU)
/// ```mermaid
/// sequenceDiagram
/// 	autonumber
/// 	participant C as Connected Client
/// 	participant S as Server
/// 	participant CAll as All Other Clients
/// 	C->>S: Handshake Identifier
/// 	C->>S: Client's Public Key
/// 	Note over S: Calculate client's unique ID
/// 	Note over S: Read account data
/// 	Note over S: Validate public key
/// 	C->>S: Display Name
/// 	Note over S: update display name
/// 	Note over S: generate random token
/// 	S->>C: Authentication Token
/// 	Note over C: sign token
/// 	C->>S: Signed Token
/// 	Note over S: Verify signed token against public key
/// 	S->>C: Notify verification status
/// 	S->>C: End Stream
/// 	alt if passed authentication
/// 		Note over S: Save user data
/// 		Note over S: Trigger Client Authenticate Event
/// 		Note over S: Create entity for client
/// 		par Server to Incoming
/// 			Note over C: Transition To InGame
/// 			S->>C: Client Joined
/// 		and Server to Others
/// 			S->>CAll: Client Joined
/// 			Note over CAll: Stub
/// 		end
/// 	else if failure
/// 		S->>C: Connection Closed
/// 		Note over C: Transition To MainMenu
/// 	end
/// ```
pub struct Identifier {
	/// The application context for the client/sender.
	pub client: Arc<client::AppContext>,
	/// The application context for the server/receiver.
	pub server: Arc<server::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = client::AppContext;
	type RecvBuilder = server::AppContext;
	fn unique_id() -> &'static str {
		"handshake"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.client
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.server
	}
}
