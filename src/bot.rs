use crate::event::{self, Payload, Event, Channel};
use crate::command::{parse_message, Action};
use crate::user::{User, DiscordUser, Member, UserVar};

use std::time::Duration;
use std::collections::HashMap;
use std::iter::Enumerate;

use failure::Error;

use reqwest::{header, Client};

use futures::{Future, Poll, Async, Stream};
use futures::sync::mpsc::{self, Sender, Receiver};
use futures::sink::Sink;

use websocket::{ClientBuilder, OwnedMessage};

use tokio::timer::Interval;

use serde_json::json;

pub struct LoftBot {
    quit: bool,
    id: String,
    guild_id: String,
    users: Vec<User>,
    user_map: HashMap<String, usize>,
    token: String,
    client: Client,
    gateway: String,
    sequence: Option<usize>,
    stream: Receiver<Event>,
    heartbeat_sender: Option<Sender<Event>>,
    message_sender: Sender<OwnedMessage>,
    channels: Vec<Channel>
}

impl LoftBot {
    pub fn run(guild_id: String) -> impl Future<Item=(), Error=failure::Error> {
        let token = String::from("Bot NTEyMDgxODQ0MzQzMTQ0NDU4.Dxp1hw.7iC-_L8jx8Mf3A8RK3K7IRFQd4w");
        futures::future::result(LoftBot::prepare_gateway(&token)).and_then(|(client, gateway, cb)| {
            cb.async_connect_secure(None)
            .from_err()
            .and_then(|(s, _)| {
                let (writer, reader) = s.split();
                let (tx, rx) = mpsc::channel(1024);
                let (stx, srx) = mpsc::channel(1024);
                let bot = LoftBot {
                    quit: false,
                    id: String::from("0"),
                    guild_id,
                    users: vec!(),
                    user_map: HashMap::new(),
                    token,
                    client,
                    gateway,
                    sequence: None,
                    stream: rx,
                    heartbeat_sender: Some(tx.clone()),
                    message_sender: stx,
                    channels: vec!(),
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
                    srx.map_err(|_| failure::err_msg("stream receive error"))
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
    pub fn get_channels(&self) -> Result<Vec<Channel>, Error> {
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/channels", self.guild_id);
        let mut res: Vec<Channel> = self.client.get(url).send()?.json()?;
        res.retain(|x| x.ty == 0);
        Ok(res)
    }
    pub fn get_online_members(&self) -> Result<Vec<DiscordUser>, Error> {
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/members", self.guild_id);
        let res: Vec<Member> = self.client.get(url).send()?.json()?;
        Ok(res.into_iter().map(|x| x.user ).collect())
    }
    pub fn create_message(&self, message: event::OutgoingMessage, channel_id: String) -> Result<(), Error> {
        let url = &format!("https://discordapp.com/api/v6/channels/{}/messages", channel_id);
        self.client.post(url).json(&message).send()?;
        Ok(())
    }
    fn send_payload(&self, pl: &Payload) -> Result<(), Error> {
        let body = serde_json::to_string(&pl)?;
        tokio::spawn(
            self.message_sender
                .clone()
                .send(OwnedMessage::Text(body))
                .map(|_| ())
                .map_err(|e| println!("send err: {}", e))
        );
        Ok(())
    }
    fn quit(&mut self) {
        println!("quitting");
        self.quit = true;
    }
    fn change_var(&mut self, var: UserVar, cmd: Option<String>, val: Option<String>, user_index: usize, channel_id: String) -> Result<(), Error> {
        match var {
            UserVar::FavFood => {
                match cmd.as_ref().map(|x| x.as_str()) {
                    Some("set") => {
                        match val {
                            Some(v) => {
                                self.users[user_index].favorite_food = Some(v);
                            },
                            None => {}
                        }
                    },
                    None => {
                        let m = match &self.users[user_index].favorite_food {
                            Some(f) => format!("Your favorite food is {}", f),
                            None => format!("You have no favorite food"),
                        };
                        self.create_message(event::OutgoingMessage {content: m}, channel_id)?;
                    }
                    _ => {}
                }
            },
            /*UserVar::Nicks => {
                match cmd.as_ref().map(|x| x.as_str()) {
                    Some("add") => {
                        match val {
                            Some(v) => {
                                self.users[user_index].notif_location_default = None;
                            },
                            None => {}
                        }
                    },
                    None => {
                        let m = match &self.users[user_index].favorite_food {
                            Some(f) => format!("Your favorite food is {}", f),
                            None => format!("Your favorite food is "),
                        };
                        self.create_message(event::OutgoingMessage {content: m}, channel_id)?;
                    }
                    _ => {}
                } 
            },*/
            UserVar::None => {},
            _ => {},
        }
        Ok(())
    }
    fn handle_message(&mut self, message: event::Message) -> Result<(), Error> {
        match parse_message(message.content) {
            Action::SendMessage(m) => self.create_message(event::OutgoingMessage {content: m}, message.channel_id)?,
            Action::ChangeVariable(var, cmd, val) => {
                return match self.user_map.get(&message.author.id) {
                    Some(i) => self.change_var(var, cmd, val, *i, message.channel_id),
                    None => Ok(()),
                };
            },
            Action::Quit => self.quit(),
            Action::None => {},
        }
        /*let m = event::OutgoingMessage {
            content: format!("Hello <@!{}>", message.author.id),
        };*/
        
        Ok(())
    }
}

impl Future for LoftBot {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if self.quit { return Ok(Async::Ready(())) }
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
                                    e.sink_map_err(|x| failure::Error::from(x))
                                    .send_all(Interval::new(tokio::clock::now(), Duration::from_millis(data.heartbeat_interval))
                                    .map(|_| Event::SendHeartbeat_))
                                    .map(|_| ()).map_err(|e| println!("timer err: {}", e))
                                );
                        }
                        None => (),
                    }
                    let ident = Payload {
                        op: 2,
                        d: json!( {
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
                Event::Ack => println!("Ack"),
                Event::EventReady(data) => self.id = data.user.id,
                Event::EventMessage(message) => if message.author.id != self.id {self.handle_message(message)?},
                Event::EventGuildCreate(guild) => {
                    self.users = guild.members.into_iter().map(|x| User::from_discord(x.user)).collect();
                    for (i, user) in self.users.iter().enumerate() {
                        self.user_map.insert(user.discord_user.id.clone(), i);
                    }
                    self.channels = guild.channels;
                },
                Event::EventChannelCreate(channel) => self.channels.push(channel),
                Event::SendHeartbeat_ | Event::Heartbeat => {
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
                Event::UnknownEvent(e) => println!("Got unhandled event {}", e),
                Event::Unknown(n) => println!("Other event : {}", n),
            }
        }
    }
}