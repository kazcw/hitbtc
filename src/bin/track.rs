use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;

use chrono::prelude::Local;
use colored::{Color, Colorize};
use decimx::DecimX;
use failure::Fail;
use futures::{Future, Sink, Stream};
use log::{debug, info, log, trace, warn};
use simble::symbol;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

use hitbtc::message::{
    ClientEnvelope, ClientMessage, Envelope, ServerCommand, SnapshotOrderbook, UpdateOrderbook,
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

    let mut pair = args
        .next()
        .expect("expected market argument (e.g. XMRBTC)")
        .parse()
        .unwrap();
    let mut byvol = false;
    if pair == symbol("-v") {
        byvol = true;
        pair = args
            .next()
            .expect("expected market argument (e.g. XMRBTC)")
            .parse()
            .unwrap();
    }
    let sub_req = serde_json::to_string(&Envelope {
        body: ServerCommand::SubscribeOrderbook { symbol: pair },
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
        .and_then(move |ws| {
            debug!("subscribed");
            ws.map_err(|e| Error::Tungstenite(e))
                .and_then(handle_message)
                .for_each(move |m| {
                    if let Some(m) = m {
                        trace!("message: {:?}", m);
                        update_book(m, &mut book, byvol);
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

fn update_book(m: ClientMessage, book: &mut Book, byvol: bool) {
    match m {
        ClientMessage::SnapshotOrderbook(SnapshotOrderbook { ask, bid, .. }) => {
            book.bids = bid.into_iter().map(|o| (o.price, o.size)).collect();
            book.asks = ask.into_iter().map(|o| (o.price, o.size)).collect();
            let bestbid = book.bids.iter().next_back().map(|o| (*o.0, *o.1)).unwrap();
            let bestask = book.asks.iter().next().map(|o| (*o.0, *o.1)).unwrap();
            if byvol {
                println!(
                    "{} {} {} {} {}",
                    Local::now(),
                    format!("{}", bestbid.1),
                    format!("{}", bestbid.0),
                    format!("{}", bestask.0),
                    format!("{}", bestask.1),
                );
            } else {
                println!("{} {} {}", Local::now(), bestbid.0, bestask.0);
            }
        }
        ClientMessage::UpdateOrderbook(UpdateOrderbook { ask, bid, .. }) => {
            let bestbid0 = book.bids.iter().next_back().map(|o| (*o.0, *o.1));
            let bestask0 = book.asks.iter().next().map(|o| (*o.0, *o.1));
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
            let bestbid1 = book.bids.iter().next_back().map(|o| (*o.0, *o.1));
            let bestask1 = book.asks.iter().next().map(|o| (*o.0, *o.1));
            // good grief, surely there's a better way
            if let Some(bestbid0) = bestbid0 {
                if let Some(bestask0) = bestask0 {
                    if let Some(bestbid1) = bestbid1 {
                        if let Some(bestask1) = bestask1 {
                            if byvol {
                                if bestbid1 != bestbid0 || bestask1 != bestask0 {
                                    if bestbid1.0 != bestbid0.0 || bestask1.0 != bestask0.0 {
                                        println!(
                                            "{} {} {} {} {}",
                                            Local::now(),
                                            format!("{}", bestbid1.1),
                                            format!("{}", bestbid1.0).color(cmpcolor(bestbid1.0, bestbid0.0)),
                                            format!("{}", bestask1.0).color(cmpcolor(bestask1.0, bestask0.0)),
                                            format!("{}", bestask1.1),
                                        );
                                    } else {
                                        println!(
                                            "{} {} {} {} {}",
                                            Local::now(),
                                            format!("{}", bestbid1.1).color(cmpcolor(bestbid1.1, bestbid0.1)),
                                            format!("{}", bestbid1.0),
                                            format!("{}", bestask1.0),
                                            format!("{}", bestask1.1).color(cmpcolor(bestask0.1, bestask1.1)),
                                        );
                                    }
                                }
                            } else {
                                if bestbid1.0 != bestbid0.0 || bestask1.0 != bestask0.0 {
                                    println!(
                                        "{} {} {}",
                                        Local::now(),
                                        format!("{}", bestbid1.0).color(cmpcolor(bestbid1.0, bestbid0.0)),
                                        format!("{}", bestask1.0).color(cmpcolor(bestask1.0, bestask0.0))
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
