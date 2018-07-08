use decimx::DecimX;
use serde_derive::{Deserialize, Serialize};
use simble::Symbol;

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ServerCommand {
    SubscribeOrderbook { symbol: Symbol },
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct Envelope<T> {
    #[serde(flatten)]
    pub body: T,
    pub id: u64,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Order {
    pub price: DecimX,
    pub size: DecimX,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SnapshotOrderbook {
    pub ask: Vec<Order>,
    pub bid: Vec<Order>,
    pub symbol: Symbol,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateOrderbook {
    pub ask: Vec<Order>,
    pub bid: Vec<Order>,
    pub symbol: Symbol,
    // sequence: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    SnapshotOrderbook(SnapshotOrderbook),
    UpdateOrderbook(UpdateOrderbook),
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientError {
    pub message: String,
    pub code: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ClientEnvelope {
    Message(ClientMessage),
    Reply { result: bool, id: u64 },
    Error { error: ClientError, id: Option<u64> },
}
