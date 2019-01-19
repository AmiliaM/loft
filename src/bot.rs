use failure::Error;
use reqwest::{header, Client};

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
}

impl LoftBot {
    pub fn new(guild_id: String) -> Result<LoftBot, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, 
            header::HeaderValue::from_static("DiscordBot (bogodynamics.io, 1.0)"));
        let token = String::from("Bot NTEyMDgxODQ0MzQzMTQ0NDU4.Dxp1hw.7iC-_L8jx8Mf3A8RK3K7IRFQd4w");
        headers.insert(header::AUTHORIZATION, 
            header::HeaderValue::from_str(&token)?);
        let client = Client::builder()
            .default_headers(headers)
            .build()?;
        let gateway = LoftBot::get_gateway(&client)?;
        Ok(LoftBot {
            client,
            guild_id,
            token,
            gateway,
            sequence: None,
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
}
