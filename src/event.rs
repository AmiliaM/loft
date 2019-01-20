#[derive(Deserialize, Serialize)]
pub struct Payload {
    pub op: i8,
    pub d: serde_json::Value,
    pub s: Option<usize>, 
    pub t: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadyMsg {
    pub session_id: String,
}

#[derive(Deserialize, Debug)]
pub struct Message {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloMsg {
    pub heartbeat_interval: u64,
}

//Both internal and extenal events or message content types
#[derive(Debug)]
pub enum Event {
    EventReady(ReadyMsg),
    EventMessage(Message),
    Hello(HelloMsg), 
    Ack,
    Heartbeat,
    SendHeartbeat_,
    Unknown(i8),
    UnknownEvent(String),
}

impl Event {
    pub fn from_payload(p: Payload) -> Result<Event,failure::Error> {
        Ok(match p.op {
            0 => match p.t.as_ref().map(|x| x.as_str()) {
                Some("READY") => Event::EventReady(serde_json::from_value(p.d)?),
                Some("MESSAGE_CREATE") => Event::EventMessage(serde_json::from_value(p.d)?),
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
