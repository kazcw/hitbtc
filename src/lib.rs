//use std::collections::{BTreeMap, HashMap};
//use std::time::Duration;
use std::io;

use failure::Fail;
use log::{debug, info, log, warn};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

use legacies::{Future, Sink, Stream};

mod message;
use crate::message::{
    ClientEnvelope, ClientMessage, Envelope, Order, ServerCommand, SnapshotOrderbook, Symbol,
    UpdateOrderbook,
};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "websockets error: {}", _0)]
    Tungstenite(#[fail(cause)] tungstenite::error::Error),
    #[fail(display = "server error: code={} message={}", code, message)]
    Server { message: String, code: u64 },
}

/*
impl Book {
}

impl Client {
    pub fn connect() -> Self;
    pub fn subscribe(Symbol) -> impl Future<Stream<BookUpdate>>;
}
*/

fn main() {
    env_logger::init();

    let sub_req = serde_json::to_string(&Envelope {
        body: ServerCommand::SubscribeOrderbook {
            symbol: Symbol::XMRBTC,
        },
        id: 1,
    }).unwrap();
    let sub_req = Message::Text(sub_req);

    let url = url::Url::parse("wss://api.hitbtc.com/api/2/ws").unwrap();
    let client = connect_async(url)
        .map_err(|e| Error::Tungstenite(e))
        .and_then(|(ws, _response)| {
            debug!("connected");
            ws.send(sub_req).map_err(|e| Error::Tungstenite(e))
        })
        .and_then(|ws| {
            debug!("subscribed");
            ws.map_err(|e| Error::Tungstenite(e))
                .for_each(handle_message)
        })
        .map_err(|e| eprintln!("{:?}", e))
        .map(|_| ())
        .map_err(|_| ());

    tokio::runtime::run(client);
}

fn handle_message(m: Message) -> Result<(), Error> {
    match m {
        Message::Text(txt) => {
            match serde_json::from_str(&txt) {
                Ok(x) => handle(x),
                Err(_) => {
                    warn!("got unknown message: {}", txt);
                    Ok(())
                }
            }
        }
        Message::Binary(bin) => {
            info!("got binary: {:?}", bin);
            Ok(())
        }
        Message::Ping(ping) => {
            info!("got ping: {:?}", ping);
            Ok(())
        }
        Message::Pong(pong) => {
            info!("got pong: {:?}", pong);
            Ok(())
        }
    }
}

fn handle(m: ClientEnvelope) -> Result<(), Error> {
    match m {
        ClientEnvelope::Message(m) => {
            info!("got message: {:?}", m);
            match m {
                ClientMessage::SnapshotOrderbook(SnapshotOrderbook { ask, bid, symbol }) => {
                    Ok(())
                }
                ClientMessage::UpdateOrderbook(UpdateOrderbook { ask, bid, symbol }) => {
                    Ok(())
                }
            }
        }
        ClientEnvelope::Reply { result, .. } => {
            info!("got reply: {}", result);
            Ok(())
        }
        ClientEnvelope::Error { error, .. } => {
            let (message, code) = (error.message, error.code);
            Err(Error::Server { message, code })
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main();
    }
}

/*
    for o in msg.bid.into_iter() {
        if o.size == "0" {
            self.bid.remove(&o.price);
        } else {
            self.bid.insert(o.price, o.size);
        }
    }
    for o in msg.ask.into_iter() {
        if o.size == "0" {
            self.ask.remove(&o.price);
        } else {
            self.ask.insert(o.price, o.size);
        }
    }
*/
