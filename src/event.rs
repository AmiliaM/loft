use crate::user::{Member, DiscordUser};
use crate::irc;

#[derive(Deserialize)]
pub struct Guild {
    pub members: Vec<Member>,
    pub channels: Vec<Channel>
}

#[derive(Deserialize)]
pub struct Channel {
    name: String,
    id: String,
    #[serde(rename = "type")]
    pub ty: i8,
}

#[derive(Deserialize, Serialize)]
pub struct Payload {
    pub op: i8,
    pub d: serde_json::Value,
    pub s: Option<usize>, 
    pub t: Option<String>,
}

#[derive(Deserialize)]
pub struct ReadyMsg {
    pub session_id: String,
    pub user: DiscordUser, 
}

#[derive(Deserialize)]
pub struct Message {
    pub content: String,
    pub channel_id: String,
    pub author: DiscordUser,
}

#[derive(Serialize)]
pub struct OutgoingMessage {
    pub content: String,
}

#[derive(Deserialize)]
pub struct HelloMsg {
    pub heartbeat_interval: u64,
}

//Both internal and extenal events or message content types
pub enum Event {
    Hello(HelloMsg), 
    Heartbeat,
    Ack,
    EventReady(ReadyMsg),
    EventMessage(Message),
    EventGuildCreate(Guild),
    EventChannelCreate(Channel),
    UnknownEvent(String),
    SendHeartbeat_,
    IRCEvent(irc::Event),
    Unknown(i8),
}

impl Event {
    pub fn from_payload(p: Payload) -> Result<Event,failure::Error> {
        Ok(match p.op {
            0 => match p.t.as_ref().map(|x| x.as_str()) {
                Some("READY") => Event::EventReady(serde_json::from_value(p.d)?),
                Some("MESSAGE_CREATE") => Event::EventMessage(serde_json::from_value(p.d)?),
                Some("GUILD_CREATE") => Event::EventGuildCreate(serde_json::from_value(p.d)?),
                Some("CHANNEL_CREATE") => Event::EventChannelCreate(serde_json::from_value(p.d)?),
                Some(e) => Event::UnknownEvent(e.to_string()),
                None => Event::Unknown(0),
            }
            1 => Event::Heartbeat,
            10 => Event::Hello(serde_json::from_value(p.d)?),
            11 => Event::Ack,
            n => Event::Unknown(n),
        })
    }
}
