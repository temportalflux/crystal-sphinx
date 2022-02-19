use std::sync::Arc;

use socknet::stream;

use crate::common::network::replication::world::chunk::{client, server};

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The identifier struct for the world-chunk streams (`replication::chunk-data`).
///
/// [Edit Diagram](https://mermaid.live/edit/#pako:eNptkk1vAiEQhv_KhJNNbGL6FcPBQ1eT9uKheyVpWJhV4i5sYdBa438vrB-trQcykPfhHWaYHVNOI-Ms4EdEq3Bq5MLLVlhBMpKzsa3Q51MnPRllOmkJSpABSvTr_1KRpaIxaClLjXMdvKFCs0YNahntCsiBx64xShJmRpBHReAX1eBuNITHtMajm4MiaO4IwaVMUHIIfU7OS7Q6x403hO-97YkvbyeTgkPRp1LOeW1sSgQD8_QAn_dXfBOt-gdz_iKtbrJx553CEK5az_uegKuhapxaBTAWaIlwwfaFo1TLA5SZC_3s9tzLrq4DEgzi-Pcb_0Cv0wQE84VnILXhaj0zm34zIljc_DS9QtAmdI3c4vHW8XoKbMha9K00Os3CTlgAwVJNLQrG01ZLvxJM2H3iYqdTP2fakPOM17IJOGR5VsqtVYyTj3iCjsN0pPbfKVPduw)
/// ```mermaid
/// sequenceDiagram
/// 	autonumber
/// 	participant S as Server
/// 	participant C as Client
/// 	loop Received chunk to replicate
/// 		rect rgb(20, 50, 80)
/// 			Note over S: server::Sender::write_chunk
/// 			S->>C: Chunk coordinate (i64 x3)
/// 			Note over C: client::Handler::process_chunk
/// 			S->>C: Number of blocks in the chunk
/// 			loop each block in chunk
/// 				S->>C: Block offset (u8 x3)
/// 				S->>C: Block ID (usize)
/// 			end
/// 			Note over C: Enqueue new chunk to be displayed
/// 		end
/// 	end
/// ```
pub struct Identifier {
	/// The (empty) application context for the server/sender.
	pub server: Arc<server::AppContext>,
	/// The application context for the client/receiver.
	pub client: Arc<client::AppContext>,
}

impl stream::Identifier for Identifier {
	type SendBuilder = server::AppContext;
	type RecvBuilder = client::AppContext;
	fn unique_id() -> &'static str {
		"replication::chunk-data"
	}
	fn send_builder(&self) -> &Arc<Self::SendBuilder> {
		&self.server
	}
	fn recv_builder(&self) -> &Arc<Self::RecvBuilder> {
		&self.client
	}
}
