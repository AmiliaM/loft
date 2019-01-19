#[macro_use] extern crate serde_derive;
use serde_json::json;

use websocket::{ClientBuilder, OwnedMessage};

use futures::future::Future;
use futures::stream::Stream;
use futures::sync::mpsc;
use futures::sink::Sink;

use tokio::timer::Interval;
use std::time::Duration;

mod bot;
use self::bot::LoftBot;

#[derive(Debug, Deserialize)]
struct ReadyMsg {
    session_id: String,
}

#[derive(Deserialize, Debug)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct HelloMsg {
    heartbeat_interval: u64,
}

//Both internal and extenal events or message content types
#[derive(Debug)]
enum Event {
    EventReady(ReadyMsg),
    EventMessage(Message),
    Hello(HelloMsg), 
    Ack,
    Heartbeat,
    SendHeartbeat_,
    Unknown,
    UnknownEvent(String),
}

impl Event {
    fn from_payload(p: Payload) -> Result<Event,failure::Error> {
        Ok(match p.op {
            0 => match p.t.as_ref().map(|x| x.as_str()) {
                Some("READY") => Event::EventReady(serde_json::from_value(p.d)?),
                Some("MESSAGE_CREATE") => Event::EventMessage(serde_json::from_value(p.d)?),
                Some(e) => Event::UnknownEvent(e.to_string()),
                None => Event::Unknown,
            }
            1 => Event::Heartbeat,
            10 => Event::Hello(serde_json::from_value(p.d)?),
            11 => Event::Ack,
            _ => Event::Unknown,
        })
    }
}

//top level structure for gateway message
#[derive(Deserialize, Serialize)]
struct Payload {
    op: i8,
    d: serde_json::Value,
    s: Option<usize>, 
    t: Option<String>,
}

fn main() -> Result<(), failure::Error> {
    let guild_id: String = String::from("533354016818593846");
    let mut bot = LoftBot::new(guild_id)?;
    let runner = ClientBuilder::new(&bot.gateway)?
        .async_connect_secure(None)
        .map_err(|x| failure::Error::from(x))
        .and_then(move |(c, _)| {
            let (w, reader) = c.split();
            let (tx, rx) = mpsc::channel(1024);
            let (stx, srx) = mpsc::channel(1024);
            let tx2 = tx.clone();
            //prints and sends all messages in srx to the gateway
            tokio::spawn(
                srx.inspect(|x| println!("send: {:?}", x))
                    .fold(w, |w, x| w.send(x).map_err(|x| println!("send err: {}", x)))
                    .map(|_| ())
            );
            tokio::spawn(
                //Event loop
                rx.map_err(|_| failure::err_msg("stream err")).for_each(move |event| {
                    let tx2 = tx2.clone();
                    let stx = stx.clone();
                    match event {
                        Event::Hello(data) => {
                            tokio::spawn(
                                Interval::new(tokio::clock::now(), Duration::from_millis(data.heartbeat_interval))
                                    .map(|_| Event::SendHeartbeat_)
                                    .map_err(|x| failure::Error::from(x)).forward(tx2)
                                    .map(|_| ()).map_err(|e| println!("timer err: {}", e))
                            );
                            let ident = Payload {
                                op: 2,
                                d: json!(
                                    {
                                        "token": bot.token,
                                        "properties": {
                                            "$os": "macos",
                                            "$browser": "loft",
                                            "$device": "loft"
                                        },
                                        "compress": false,
                                    }
                                ),
                                s: None,
                                t: None,
                            }; 
                            let body = serde_json::to_string(&ident)?;
                            tokio::spawn(
                                stx.send(OwnedMessage::Text(body)).map(|_| ()).map_err(|e| println!("send err: {}", e))
                            );
                        },
                        Event::Heartbeat => println!("Received heartbeat"),
                        Event::SendHeartbeat_ => {
                            let hb = Payload {
                                op: 1,
                                d: match bot.sequence {
                                    None => serde_json::Value::Null,
                                    Some(n) => serde_json::Value::Number(n.into()),
                                },
                                s: None,
                                t: None,
                            }; 
                            let body = serde_json::to_string(&hb)?;
                            tokio::spawn(
                                stx.send(OwnedMessage::Text(body)).map(|_| ()).map_err(|e| println!("send err: {}", e))
                            );
                        },
                        Event::EventReady(r) => println!("Session id: {} ready", r.session_id),
                        Event::Ack => println!("Ack"),
                        Event::UnknownEvent(e) => println!("Got unhandled event {}", e),
                        _ => {println!("other message")}
                    }
                    Ok(())
                }).map_err(|e| println!("rx err: {}", e))
            );

            //take message from reader and put text messages into tx
            reader.from_err()
                .and_then(|message| {
                    match message {
                        OwnedMessage::Text(text) => {
                            //println!("{}", text);
                            let res: Payload = serde_json::from_str(&text)?; 
                            Ok(Some(Event::from_payload(res)?))
                        }
                        _ => { println!("Non-text message {:?} received", message); Ok(None) }
                    }
                })
                .filter_map(|x| x)
                .forward(tx)
                .map(|_| ())
        }).or_else(|e| {
            println!("error: {}", e);
            Ok(())
        });
    tokio::run(runner);
    Ok(())
}
