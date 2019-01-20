use crate::event::{Payload, Event};

use std::time::Duration;

use failure::Error;

use reqwest::{header, Client};

use futures::{Future, Poll, Async, Stream};
use futures::sync::mpsc::{self, Sender, Receiver};
use futures::sink::Sink;

use websocket::{ClientBuilder, OwnedMessage};

use tokio::timer::Interval;

use serde_json::json;


#[derive(Deserialize)]
pub struct DiscordUser {
    username: String,
    discriminator: String,
    id: String,
}

#[derive(Deserialize)]
pub struct Channel {
    name: String,
    id: String,
    #[serde(rename = "type")]
    ty: i8,
}

pub struct LoftBot {
    client: Client,
    guild_id: String,
    pub token: String,
    pub gateway: String,
    pub sequence: Option<usize>,
    stream: Receiver<Event>,
    heartbeat_sender: Option<Sender<Event>>,
    message_sender: Sender<OwnedMessage>,

}

fn prepare_gateway(token: &str) -> Result<(Client, String, ClientBuilder<'static>), failure::Error> {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, 
        header::HeaderValue::from_static("DiscordBot (bogodynamics.io, 1.0)"));
    headers.insert(header::AUTHORIZATION, 
        header::HeaderValue::from_str(token)?);
    let client = Client::builder()
        .default_headers(headers)
        .build()?;
    let gateway = LoftBot::get_gateway(&client)?;
    let cb = ClientBuilder::new(&gateway)?;
    Ok((client, gateway, cb))
}

impl LoftBot {
    pub fn run(guild_id: String) -> impl Future<Item=(), Error=failure::Error> {
        let token = String::from("Bot NTEyMDgxODQ0MzQzMTQ0NDU4.Dxp1hw.7iC-_L8jx8Mf3A8RK3K7IRFQd4w");
        futures::future::result(prepare_gateway(&token)).and_then(|(client, gateway, cb)| {
            cb.async_connect_secure(None)
            .from_err()
            .and_then(|(s, _)| {
                let (writer, reader) = s.split();
                let (tx, rx) = mpsc::channel(1024);
                let (stx, srx) = mpsc::channel(1024);
                let bot = LoftBot {
                    client,
                    guild_id,
                    token,
                    gateway,
                    sequence: None,
                    stream: rx,
                    heartbeat_sender: Some(tx.clone()),
                    message_sender: stx,
                };
                tokio::spawn(reader.map_err(|x| failure::Error::from(x))
                    .and_then(|message| {
                        match message {
                            OwnedMessage::Text(text) => {
                                let res: Payload = serde_json::from_str(&text)?; 
                                Ok(Some(Event::from_payload(res)?))
                            }
                            _ => { println!("Non-text message {:?} received", message); Ok(None) }
                        }
                    })
                    .filter_map(|x| x)
                    .forward(tx)
                    .map(|_| ())
                    .map_err(|x| println!("read err: {}", x))
                );
                tokio::spawn(
                    srx.map_err(|x| failure::err_msg("stream receive error"))
                    .forward(writer)
                    .map(|_| ())
                    .map_err(|e| println!("{}", e))
                );
                bot
            })
        })
    }
    fn get_gateway(client: &Client) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct Gateway {
            url: String,
        }
        let gw: Gateway = client.get("https://discordapp.com/api/v6/gateway")
            .send()?
            .json()?;
        let url = format!("{}/?v=6&encoding=json", gw.url);
        Ok(url)
    }
    pub fn get_channels(&self) -> Result<Vec<Channel>, Error> {
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/channels", self.guild_id);
        let mut res: Vec<Channel> = self.client.get(url).send()?.json()?;
        res.retain(|x| x.ty == 0);
        Ok(res)
    }
    pub fn get_online_members(&self) -> Result<Vec<DiscordUser>, Error> {
        #[derive(Deserialize)]
        struct Member {
            user: DiscordUser
        } 
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/members", self.guild_id);
        let res: Vec<Member> = self.client.get(url).send()?.json()?;
        Ok(res.into_iter().map(|x| x.user ).collect())
    }
    pub fn create_message(&self, content: String, channel_id: String) -> Result<(), Error> {
        let url = &format!("https://discordapp.com/api/v6/channes/{}/messages", channel_id);
        #[derive(Serialize)]
        struct Message {
            content: String,
        }
        let m = Message {content};
        let body = serde_json::to_string(&m)?;
        let res = self.client.post(url).form(&body).send()?.text()?;
        println!("{}", res);
        Ok(())
    }
    fn send_payload(&self, pl: &Payload) -> Result<(), Error> {
        let body = serde_json::to_string(&pl)?;
        tokio::spawn(
            self.message_sender.clone().send(OwnedMessage::Text(body)).map(|_| ()).map_err(|e| println!("send err: {}", e))
        );
        Ok(())
    }
}

impl Future for LoftBot {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let event = match self.stream.poll() {
                Ok(Async::Ready(Some(e))) => e,
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_) => return Err(failure::err_msg("stream receive error")),
            };
            match event {
                Event::Hello(data) => {
                    match self.heartbeat_sender.take() {
                        Some(e) => {
                                tokio::spawn(
                                    Interval::new(tokio::clock::now(), Duration::from_millis(data.heartbeat_interval))
                                    .map(|_| Event::SendHeartbeat_)
                                    .map_err(|x| failure::Error::from(x)).forward(e)
                                    .map(|_| ()).map_err(|e| println!("timer err: {}", e))
                                );
                        }
                        None => (),
                    }
                    let ident = Payload {
                        op: 2,
                        d: json!(
                            {
                                "token": self.token,
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
                    self.send_payload(&ident)?;
                },
                Event::Heartbeat => println!("Received heartbeat"),
                Event::SendHeartbeat_ => {
                    let hb = Payload {
                        op: 1,
                        d: match self.sequence {
                            None => serde_json::Value::Null,
                            Some(n) => serde_json::Value::Number(n.into()),
                        },
                        s: None,
                        t: None,
                    }; 
                    self.send_payload(&hb)?;
                },
                Event::EventReady(r) => println!("Session id: {} ready", r.session_id),
                Event::Ack => println!("Ack"),
                Event::UnknownEvent(e) => println!("Got unhandled event {}", e),
                Event::Unknown(n) => println!("Other event : {}", n),
                _ => {println!("other message")}
            }
        }
    }
}