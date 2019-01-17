#[macro_use] extern crate serde_derive;
use websocket::{ClientBuilder, OwnedMessage};
use futures::future::{self, Future};
use futures::stream::Stream;
use futures::sync::mpsc;
use futures::sink::Sink;
use tokio::timer::Interval;
mod api;
use std::time::Duration;
use self::api::LoftBot;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct ReadyMsg {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct HelloMsg {
    heartbeat_interval: usize,
}

#[derive(Debug)]
enum Event {
    Ready(ReadyMsg),
    Hello(HelloMsg), 
    Heartbeat,
    SendHeartbeat,
    Unknown,
}

impl Event {
    fn from_payload(p: Payload) -> Result<Event,failure::Error> {
        Ok(match p.op {
            10 => Event::Hello(serde_json::from_value(p.d)?),
            _ => Event::Unknown,
        })
    }
}

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
    println!("{}", bot.gateway);
    //let result = bot.get_channels()?;
    //println!("{}", result);
    let runner = ClientBuilder::new(&bot.gateway)?
        .async_connect_secure(None)
        .map_err(|x| failure::Error::from(x))
        .and_then(move |(c, _)| {
            let (w, reader) = c.split();
            let (tx, rx) = mpsc::channel(1024);
            let (stx, srx) = mpsc::channel(1024);
            let tx2 = tx.clone();
            tokio::spawn(
                srx.inspect(|x| println!("send: {:?}", x)).fold(w, |w, x| w.send(x).map_err(|x| println!("send err: {}", x))).map(|_| ())
            );
            tokio::spawn(
                rx.map_err(|_| failure::err_msg("stream err")).for_each(move |event| {
                    let tx2 = tx2.clone();
                    let stx = stx.clone();
                    match event {
                        Event::Hello(data) => {
                            tokio::spawn(
                                Interval::new(tokio::clock::now(), Duration::from_secs(data.heartbeat_interval as u64)).map(|_| Event::SendHeartbeat).map_err(|x| failure::Error::from(x)).forward(tx2)
                                    .map(|_| ()).map_err(|e| println!("timer err: {}", e))
                            );
                            let ident = Payload {
                                op: 1,
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
                        Event::SendHeartbeat => {
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
                        Event::Ready(r) => {},
                        _ => {}
                    }
                    Ok(())
                }).map_err(|e| println!("rx err: {}", e))
            );

            reader.from_err()
                .and_then(|message| {
                    match message {
                        OwnedMessage::Text(text) => {
                            let res: Payload = serde_json::from_str(&text)?; 
                            Ok(Some(Event::from_payload(res)?))
                        }
                        _ => { println!("{:?}", message); Ok(None) }
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

//--Persistant gateway connection--
//Identify