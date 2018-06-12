#[macro_use]
extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;

use std::time::Duration;
use std::{io, thread};

use actix::prelude::*;
use actix_web::ws;
use futures::Future;

mod message;
use message::{
    ClientEnvelope, ClientMessage, Envelope, Order, ServerCommand, SnapshotOrderbook, Symbol,
    UpdateOrderbook,
};

pub fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=debug");
    let _ = env_logger::init();
    let sys = actix::System::new("hitbtc");

    Arbiter::handle().spawn(
        ws::Client::new("wss://api.hitbtc.com/api/2/ws")
            .max_frame_size(1024 * 256)
            .connect()
            .map_err(|e| {
                println!("Error: {:?}", e);
                ()
            })
            .map(|(reader, writer)| {
                let addr: Addr<Syn, _> = ExchangeClient::create(|ctx| {
                    ExchangeClient::add_stream(reader, ctx);
                    ExchangeClient {
                        writer,
                        books: Default::default(),
                    }
                });

                // start console loop
                thread::spawn(move || loop {
                    let mut cmd = String::new();
                    if io::stdin().read_line(&mut cmd).is_err() {
                        println!("error");
                        return;
                    }
                    addr.do_send(Subscribe(Symbol::XMRBTC));
                });

                ()
            }),
    );

    let _ = sys.run();
}

struct Subscribe(Symbol);

impl Message for Subscribe {
    type Result = Addr<Syn, OrderBook>;
}

impl Actor for ExchangeClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("# Disconnected");

        Arbiter::system().do_send(actix::msgs::SystemExit(0));
    }
}

struct ExchangeClient {
    writer: ws::ClientWriter,
    books: HashMap<Symbol, Addr<Syn, OrderBook>>,
}

use std::collections::{BTreeMap, HashMap};

impl ExchangeClient {
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.writer.ping("");
            act.hb(ctx);
        });
    }

    fn subscribe(&mut self, symbol: Symbol, ctx: &mut Context<Self>) -> Addr<Syn, OrderBook> {
        let client: Addr<Syn, _> = ctx.address();
        let book: Addr<Syn, _> = OrderBook::create(move |c| OrderBook {
            symbol,
            client,
            bid: Default::default(),
            ask: Default::default(),
        });
        self.books.insert(symbol, book.clone());
        self.writer.text(
            serde_json::to_string(&Envelope {
                body: ServerCommand::SubscribeOrderbook { symbol },
                id: 1,
            }).unwrap(),
        );
        book
    }
}

/// Handle stdin commands
impl Handler<Subscribe> for ExchangeClient {
    type Result = Addr<Syn, OrderBook>;

    fn handle(&mut self, msg: Subscribe, ctx: &mut Context<Self>) -> Self::Result {
        self.subscribe(msg.0, ctx)
    }
}

/// Handle server websocket messages
impl StreamHandler<ws::Message, ws::ProtocolError> for ExchangeClient {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Context<Self>) {
        match msg {
            ws::Message::Text(txt) => {
                let v: ClientEnvelope = serde_json::from_str(&txt).unwrap();
                match v {
                    ClientEnvelope::Message(m) => match m {
                        ClientMessage::SnapshotOrderbook(SnapshotOrderbook {
                            ask,
                            bid,
                            symbol,
                        }) => self.books[&symbol].do_send(SetBook { ask, bid }),
                        ClientMessage::UpdateOrderbook(UpdateOrderbook { ask, bid, symbol }) => {
                            self.books[&symbol].do_send(UpdateBook { ask, bid })
                        }
                    },
                    ClientEnvelope::Reply { .. } => (),
                }
            }
            _ => (),
        }
    }

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("# Connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        println!("# Server disconnected");
        ctx.stop()
    }
}

struct OrderBook {
    symbol: Symbol,
    client: Addr<Syn, ExchangeClient>,
    bid: BTreeMap<String, String>,
    ask: BTreeMap<String, String>,
}

impl Actor for OrderBook {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {}

    fn stopped(&mut self, _: &mut Context<Self>) {}
}

#[derive(Message)]
struct SetBook {
    bid: Vec<Order>,
    ask: Vec<Order>,
}

impl Handler<SetBook> for OrderBook {
    type Result = ();

    fn handle(&mut self, msg: SetBook, ctx: &mut Context<Self>) {
        self.bid = msg.bid.into_iter().map(|o| (o.price, o.size)).collect();
        self.ask = msg.ask.into_iter().map(|o| (o.price, o.size)).collect();
    }
}

#[derive(Message)]
struct UpdateBook {
    bid: Vec<Order>,
    ask: Vec<Order>,
}

impl Handler<UpdateBook> for OrderBook {
    type Result = ();

    fn handle(&mut self, msg: UpdateBook, ctx: &mut Context<Self>) {
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
    }
}
