use crate::event::{self, Payload, Event, Channel};
use crate::command::{parse_message, Action};
use crate::user::{User, DiscordUser, Member, UserVar, NotifLocation};

use std::time::Duration;
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use crate::irc;

use failure::Error;

use serde_json::json;

use tokio::timer::Interval;

use reqwest::{header, Client};

use futures::{Future, Poll, Async, Stream};
use futures::sync::mpsc::{self, Sender, Receiver};
use futures::sync::oneshot;
use futures::sink::Sink;

use websocket::{ClientBuilder, OwnedMessage};

pub struct LoftBot {
    quit: bool,
    id: String,
    guild_id: String,
    users: Vec<User>,
    user_map: HashMap<String, usize>,
    irc_users: Vec<String>,
    channels: Vec<Channel>,
    token: String,
    client: Client,
    gateway: String,
    sequence: Option<usize>,
    stream: Receiver<Event>,
    heartbeat_sender: Option<Sender<Event>>,
    irc_sender: Sender<xirc::Command>,
    message_sender: Sender<OwnedMessage>,
    shutdown: Option<oneshot::Receiver<()>>,
    shutdowntx: Option<oneshot::Sender<()>>,
}

impl LoftBot {
    pub fn run(args: HashMap<String, String>) -> impl Future<Item=(), Error=failure::Error> {
        let token = args.get("token").expect("poppis").to_string();
        futures::future::result(LoftBot::prepare_gateway(&token)).and_then(move |(client, gateway, cb)| {
            cb.async_connect_secure(None)
            .from_err()
            .and_then(move |(s, _)| {
                let (tx, rx) = mpsc::channel(1024);
                let (irctx, ircrx) = mpsc::channel(1024);
                irc::connect(
                    args.get("host").expect("blep").to_socket_addrs().expect("blel").next().expect("hiya"), 
                    args.get("nick").expect("blelele"),
                    args.get("user").expect("melm"),
                    tx.clone(),
                    ircrx
                );
                let (writer, reader) = s.split();
                let (stx, srx) = mpsc::channel(1024);
                let (shutdowntx, shutdownrx) = oneshot::channel();
                let bot = LoftBot {
                    quit: false,
                    id: String::from("0"),
                    guild_id: args.get("guildid").expect("bleppery").to_string(),
                    users: vec!(),
                    user_map: HashMap::new(),
                    irc_users: vec!(),
                    token,
                    client,
                    gateway,
                    sequence: None,
                    stream: rx,
                    heartbeat_sender: Some(tx.clone()),
                    irc_sender: irctx,
                    message_sender: stx,
                    channels: vec!(),
                    shutdown: Some(shutdownrx),
                    shutdowntx: Some(shutdowntx),
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
                    .map_err(|x| println!("Read error: {}", x))
                );
                tokio::spawn(
                    srx.map_err(|_| failure::err_msg("stream receive error"))
                    .forward(writer)
                    .map(|_| ())
                    .map_err(|e| println!("Write error: {}", e))
                );
                println!("{:?}", bot.get_channels());
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
                .map_err(|e| println!("Send error: {}", e))
        );
        Ok(())
    }
    fn quit(&mut self) {
        println!("Quitting...");
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
            UserVar::Nick => {
                match cmd.as_ref().map(|x| x.as_str()) {
                    Some("set") => {
                        match val {
                            Some(v) => {
                                self.users[user_index].irc_name = Some(v);
                            },
                            None => {}
                        }
                    },
                    None => {
                        let m = match &self.users[user_index].irc_name {
                            Some(name) => format!("Your IRC name is {}", name),
                            None => format!("You have no IRC name"),
                        };
                        self.create_message(event::OutgoingMessage {content: m}, channel_id)?;
                    }
                    _ => {}
                }
            }
            UserVar::Notif => {
                match cmd.as_ref().map(|x| x.as_str()) {
                    Some("set") => {
                        match val.as_ref().map(|x| x.as_str()) {
                            Some("discord") => self.users[user_index].notif_location = Some(NotifLocation::Discord),
                            Some("irc") => self.users[user_index].notif_location = Some(NotifLocation::IRC),
                            None => {},
                            _ => {},
                        }
                    },
                    None => {
                        let m = match &self.users[user_index].notif_location {
                            Some(location) => format!(
                                "Notifying on {}", 
                                match location {
                                    NotifLocation::Discord => "Discord",
                                    NotifLocation::IRC => "IRC"
                                }
                            ),
                            None => format!("No notification preference set. Your notifications will be sent to discord."),
                        };
                        self.create_message(event::OutgoingMessage {content: m}, channel_id)?;
                    }
                    _ => {}
                } 
            },
            UserVar::None => {},
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
        Ok(())
    }
}

impl Future for LoftBot {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while !self.quit {
            let event = match self.stream.poll() {
                Ok(Async::Ready(Some(e))) => e,
                Ok(Async::Ready(None)) => { self.quit = true; break; },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_) => return Err(failure::err_msg("stream receive error")),
            };
            match event {
                Event::Hello(data) => {
                    match (self.heartbeat_sender.take(), self.shutdown.take()) {
                        (Some(e), Some(shutdown)) => {
                                tokio::spawn(
                                    Interval::new(tokio::clock::now(), Duration::from_millis(data.heartbeat_interval))
                                    .map(|_| Event::SendHeartbeat_)
                                    .map_err(|e| failure::Error::from(e))
                                    .forward(e)
                                    .map(|_| ())
                                    .map_err(|e| println!("Timer error: {}", e))
                                    .select(shutdown.map_err(|_| ()))
                                    .map(|_| ())
                                    .map_err(|_| ())
                                );
                        }
                        (..) => (),
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
                Event::Ack => {},
                Event::EventReady(data) => self.id = data.user.id,
                Event::EventMessage(message) => if message.author.id != self.id {self.handle_message(message)?},
                Event::EventGuildCreate(guild) => {
                    self.users = guild.members.into_iter().map(|x| User::from_discord(x.user)).collect();
                    for (i, user) in self.users.iter().enumerate() {
                        self.user_map.insert(user.discord_user.id.clone(), i);
                    }
                    self.channels = guild.channels;
                    println!("Ready");
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
                Event::IRCEvent(event) => {
                    println!("{:?}", event);
                    match event.command {
                        xirc::Command::Reply(xirc::Numeric(1)) => {
                            tokio::spawn(
                                self.irc_sender.clone().send(xirc::Command::Join(vec!("##cosi".to_string()), None)).map_err(|_| println!("IRC Send err")).map(|_| ())
                            );
                        },
                        xirc::Command::Notice(xirc::CommandTarget::User(user), notice) => println!("IRC User {} notice: {}", user, notice),
                        xirc::Command::Notice(xirc::CommandTarget::Channel(channel), notice) => println!("IRC Channel {} notice: {}", channel, notice),
                        xirc::Command::PrivMsg(xirc::CommandTarget::Channel(channel), msg) => {
                            match event.source {
                                Some(xirc::EventSource::User(hm)) => {
                                    if hm.nick != "LoftBot" {
                                        let m = format!("<{}>: {}", hm.nick, msg);
                                        self.create_message(event::OutgoingMessage {content: m}, "533354016818593850".to_string())?;
                                    }
                                },
                                _ => {},
                            }
                        },
                        xirc::Command::Ping(name, arg) => {
                            tokio::spawn(
                                self.irc_sender.clone().send(xirc::Command::Pong(name, arg)).map(|_| ()).map_err(|_| ())
                            );
                        }
                        _event => println!("Unknown IRC event: {:?}", _event),
                    }
                    
                },
                Event::UnknownEvent(e) => println!("Got unhandled event {}", e),
                Event::Unknown(n) => println!("Other event: {}", n),
            }
        }
        if let Some(tx) = self.shutdowntx.take() {
            tx.send(()).map_err(|_| failure::err_msg("failed to stop timer"))?;
        }
        Ok(Async::Ready(()))
    }
}