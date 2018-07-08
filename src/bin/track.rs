use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::rc::Rc;

use chrono::prelude::Local;
use colored::{Color, Colorize};
use failure::Fail;
use futures::{Future, Sink, Stream};
use log::{debug, info, log, trace, warn};
use simble::symbol;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use xenon::{Xe, XeNS};

use hitbtc::message::{
    ClientEnvelope, ClientMessage, Envelope, Order, Reply, ServerCommand, SnapshotOrderbook,
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
    bids: BTreeMap<XeNS, XeNS>,
    asks: BTreeMap<XeNS, XeNS>,
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

    let get_symbol = serde_json::to_string(&Envelope {
        body: ServerCommand::GetSymbol { symbol: pair },
        id: 1,
    }).unwrap();
    let get_symbol = Message::Text(get_symbol);

    let sub_req = serde_json::to_string(&Envelope {
        body: ServerCommand::SubscribeOrderbook { symbol: pair },
        id: 2,
    }).unwrap();
    let sub_req = Message::Text(sub_req);

    let mut book: Book = Default::default();
    let precision = Rc::new(Cell::new(None));
    let prec2 = Rc::clone(&precision);

    let url = url::Url::parse("wss://api.hitbtc.com/api/2/ws").unwrap();
    let client = connect_async(url)
        .map_err(|e| Error::Tungstenite(e))
        .and_then(|(ws, _response)| ws.send(get_symbol).map_err(|e| Error::Tungstenite(e)))
        .and_then(|ws| ws.send(sub_req).map_err(|e| Error::Tungstenite(e)))
        .and_then(move |ws| {
            ws.map_err(|e| Error::Tungstenite(e))
                .and_then(move |m| handle_message(m, &precision))
                .for_each(move |m| {
                    if let Some(m) = m {
                        trace!("message: {:?}", m);
                        update_book(m, &mut book, byvol, &prec2);
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

fn handle_message(
    m: Message,
    precision: &Cell<Option<(i8, i8)>>,
) -> Result<Option<ClientMessage>, Error> {
    match m {
        Message::Text(txt) => match serde_json::from_str(&txt) {
            Ok(x) => handle(x, precision),
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

fn exp(s: &str) -> Result<i8, ()> {
    let mut x = 0i32;
    let mut q = 0i32;
    for c in s.as_bytes() {
        match c {
            b'0' => {
                x += q;
            }
            b'.' => {
                if x != 0 {
                    return Err(());
                }
                x = -1;
                q = -1;
            }
            b'1' => {
                if q == 1 {
                    return Err(());
                }
                q = 1;
            }
            _ => return Err(()),
        }
    }
    Ok(x as i8)
}

fn handle(
    m: ClientEnvelope,
    precision: &Cell<Option<(i8, i8)>>,
) -> Result<Option<ClientMessage>, Error> {
    match m {
        ClientEnvelope::Message(m) => Ok(Some(m)),
        ClientEnvelope::Reply { result, .. } => {
            match result {
                Reply::GetSymbol(x) => {
                    debug!("symbol info: {:?}", x);
                    precision.set(Some((
                        exp(&x.tick_size).expect("bad tick?"),
                        exp(&x.quantity_increment).expect("bad quant inc?"),
                    )));
                }
                _ => info!("got reply: {:?}", result),
            }
            Ok(None)
        }
        ClientEnvelope::Error { error, .. } => {
            let (message, code) = (error.message, error.code);
            Err(Error::Server { message, code })
        }
    }
}

fn update_book(m: ClientMessage, book: &mut Book, byvol: bool, precision: &Cell<Option<(i8, i8)>>) {
    let (aprec, sprec) = precision.get().unwrap();
    match m {
        ClientMessage::SnapshotOrderbook(SnapshotOrderbook { ask, bid, .. }) => {
            let order = move |o: Order| {
                (
                    Xe::from_str_at_precision(&o.price, aprec).expect("bad price xe?"),
                    Xe::from_str_at_precision(&o.size, sprec).expect("bad size xe?"),
                )
            };
            book.bids = bid.into_iter().map(order).collect();
            book.asks = ask.into_iter().map(order).collect();
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
            let order = move |o: Order| {
                (
                    Xe::from_str_at_precision(&o.price, aprec).unwrap(),
                    Xe::from_str_at_precision(&o.size, sprec).unwrap(),
                )
            };
            let from_book = |o: (&XeNS, &XeNS)| (*o.0, *o.1);
            let bestbid0 = book.bids.iter().next_back().map(from_book);
            let bestask0 = book.asks.iter().next().map(from_book);
            for o in bid.into_iter().map(order) {
                if o.1.is_zero() {
                    book.bids.remove(&o.0);
                } else {
                    book.bids.insert(o.0, o.1);
                }
            }
            for o in ask.into_iter().map(order) {
                if o.1.is_zero() {
                    book.asks.remove(&o.0);
                } else {
                    book.asks.insert(o.0, o.1);
                }
            }
            let bestbid1 = book.bids.iter().next_back().map(from_book);
            let bestask1 = book.asks.iter().next().map(from_book);
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
                                            format!("{}", bestbid1.0)
                                                .color(cmpcolor(bestbid1.0, bestbid0.0)),
                                            format!("{}", bestask1.0)
                                                .color(cmpcolor(bestask1.0, bestask0.0)),
                                            format!("{}", bestask1.1),
                                        );
                                    } else {
                                        println!(
                                            "{} {} {} {} {}",
                                            Local::now(),
                                            format!("{}", bestbid1.1)
                                                .color(cmpcolor(bestbid1.1, bestbid0.1)),
                                            format!("{}", bestbid1.0),
                                            format!("{}", bestask1.0),
                                            format!("{}", bestask1.1)
                                                .color(cmpcolor(bestask0.1, bestask1.1)),
                                        );
                                    }
                                }
                            } else {
                                if bestbid1.0 != bestbid0.0 || bestask1.0 != bestask0.0 {
                                    println!(
                                        "{} {} {}",
                                        Local::now(),
                                        format!("{}", bestbid1.0)
                                            .color(cmpcolor(bestbid1.0, bestbid0.0)),
                                        format!("{}", bestask1.0)
                                            .color(cmpcolor(bestask1.0, bestask0.0))
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
