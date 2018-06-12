#[derive(Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ServerCommand {
    SubscribeOrderbook { symbol: Symbol },
}

#[derive(Serialize)]
pub struct Envelope<T> {
    pub body: T,
    pub id: u64,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Symbol {
    XMRBTC,
}

#[derive(Debug, Deserialize)]
pub struct Order {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotOrderbook {
    pub ask: Vec<Order>,
    pub bid: Vec<Order>,
    pub symbol: Symbol,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderbook {
    pub ask: Vec<Order>,
    pub bid: Vec<Order>,
    pub symbol: Symbol,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    SnapshotOrderbook(SnapshotOrderbook),
    UpdateOrderbook(UpdateOrderbook),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ClientEnvelope {
    Message(ClientMessage),
    Reply { result: bool, id: u64 },
}
