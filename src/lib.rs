//use std::sync::mpsc::channel::{self, Receiver, Sender};

use failure::Fail;
use futures::{Future, Sink, Stream};
use log::{debug, info, log, warn};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message as WsMessage;

pub mod message;
use crate::message::ClientMessage as Message;
use crate::message::{
    ClientEnvelope, Envelope, Order, ServerCommand, SnapshotOrderbook, Symbol, UpdateOrderbook,
};

/*
#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "websockets error: {}", _0)]
    Tungstenite(#[fail(cause)] tungstenite::error::Error),
    #[fail(display = "server error: code={} message={}", code, message)]
    Server { message: String, code: u64 },
}

pub struct Subscribe<T>(Symbol, Receiver<T>);

/// Returns a futures-0.1 Future that must be polled in a tokio-0.1 context, and a channel for subscription requests
pub fn connect<T: From<Message>>() -> (impl Future<Item = (), Error = Error>, Sender<Subscribe>) {
    let (tx, rx) = channel::<Subscribe>();
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
}

fn subscribe() {
    let sub_req = serde_json::to_string(&Envelope {
        body: ServerCommand::SubscribeOrderbook {
            symbol: Symbol::XMRBTC,
        },
        id: 1,
    }).unwrap();
    let sub_req = WsMessage::Text(sub_req);
}

fn handle_message(m: WsMessage) -> Result<(), Error> {
    match m {
        WsMessage::Text(txt) => match serde_json::from_str(&txt) {
            Ok(x) => handle(x),
            Err(_) => {
                warn!("got unknown message: {}", txt);
                Ok(())
            }
        },
        WsMessage::Binary(bin) => {
            info!("got binary: {:?}", bin);
            Ok(())
        }
        WsMessage::Ping(ping) => {
            info!("got ping: {:?}", ping);
            Ok(())
        }
        WsMessage::Pong(pong) => {
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
                Message::SnapshotOrderbook(SnapshotOrderbook { ask, bid, symbol }) => Ok(()),
                Message::UpdateOrderbook(UpdateOrderbook { ask, bid, symbol }) => Ok(()),
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
*/
