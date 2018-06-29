use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::env;

use chrono::prelude::Local;
use colored::{Color, Colorize};
use decimx::DecimX;
use failure::Fail;
use futures::{Future, Sink, Stream};
use log::{debug, info, log, warn};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

use hitbtc::message::{
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

#[derive(Default)]
struct Book {
    bids: BTreeMap<DecimX, DecimX>,
    asks: BTreeMap<DecimX, DecimX>,
}

fn cmpcolor<T: Ord>(a: T, b: T) -> Color {
    match a.cmp(&b) {
        Ordering::Greater => Color::Green,
        Ordering::Less => Color::Red,
        Ordering::Equal => Color::White,
    }
}

pub fn main() {
    env_logger::init();

    let mut args = env::args();
    args.next().unwrap();

    let symbol = match args
        .next()
        .expect("expected market argument (e.g. XMRBTC)")
        .as_ref()
    {
        "XMRBTC" => Symbol::XMRBTC,
        "LTCBTC" => Symbol::LTCBTC,
        _ => panic!("unknown market pair!"),
    };
    let sub_req = serde_json::to_string(&Envelope {
        body: ServerCommand::SubscribeOrderbook { symbol },
        id: 1,
    }).unwrap();
    let sub_req = Message::Text(sub_req);

    let mut book: Book = Default::default();

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
                .and_then(handle_message)
                .for_each(move |m| {
                    if let Some(m) = m {
                        debug!("got message: {:?}", m);
                        update_book(m, &mut book);
                    }
                    Ok(())
                })
        })
        .map_err(|e| eprintln!("{}", e))
        .map(|_| ())
        .map_err(|_| ());

    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
    rt.spawn(client);
    rt.run().unwrap();
}

fn handle_message(m: Message) -> Result<Option<ClientMessage>, Error> {
    match m {
        Message::Text(txt) => match serde_json::from_str(&txt) {
            Ok(x) => handle(x),
            Err(_) => {
                warn!("got unknown message: {}", txt);
                Ok(None)
            }
        },
        Message::Binary(bin) => {
            info!("got binary: {:?}", bin);
            Ok(None)
        }
        Message::Ping(ping) => {
            info!("got ping: {:?}", ping);
            Ok(None)
        }
        Message::Pong(pong) => {
            info!("got pong: {:?}", pong);
            Ok(None)
        }
    }
}

fn handle(m: ClientEnvelope) -> Result<Option<ClientMessage>, Error> {
    match m {
        ClientEnvelope::Message(m) => Ok(Some(m)),
        ClientEnvelope::Reply { result, .. } => {
            info!("got reply: {}", result);
            Ok(None)
        }
        ClientEnvelope::Error { error, .. } => {
            let (message, code) = (error.message, error.code);
            Err(Error::Server { message, code })
        }
    }
}

fn update_book(m: ClientMessage, book: &mut Book) {
    match m {
        ClientMessage::SnapshotOrderbook(SnapshotOrderbook {
            ask,
            bid,
            symbol,
        }) => {
            book.bids = bid.into_iter().map(|o| (o.price, o.size)).collect();
            book.asks = ask.into_iter().map(|o| (o.price, o.size)).collect();
            let bestbid = book.bids.iter().rev().next().map(|o| o.0);
            let bestask = book.asks.iter().next().map(|o| o.0);
            println!(
                "{} {} {}",
                Local::now(),
                bestbid.unwrap(),
                bestask.unwrap()
            );
        }
        ClientMessage::UpdateOrderbook(UpdateOrderbook {
            ask,
            bid,
            symbol,
        }) => {
            let bestbid0 =
                book.bids.iter().rev().next().map(|o| o.0).unwrap().clone();
            let bestask0 =
                book.asks.iter().next().map(|o| o.0).unwrap().clone();
            for o in bid.into_iter() {
                if o.size.is_zero() {
                    book.bids.remove(&o.price);
                } else {
                    book.bids.insert(o.price, o.size);
                }
            }
            for o in ask.into_iter() {
                if o.size.is_zero() {
                    book.asks.remove(&o.price);
                } else {
                    book.asks.insert(o.price, o.size);
                }
            }
            let bestbid1 =
                book.bids.iter().rev().next().map(|o| o.0).unwrap().clone();
            let bestask1 =
                book.asks.iter().next().map(|o| o.0).unwrap().clone();
            if bestbid1 != bestbid0 || bestask1 != bestask0 {
                println!(
                    "{} {} {}",
                    Local::now(),
                    format!("{}", bestbid1).color(cmpcolor(bestbid1, bestbid0)),
                    format!("{}", bestask1).color(cmpcolor(bestask1, bestask0))
                );
            }
        }
    }
}
