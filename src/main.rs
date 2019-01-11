extern crate reqwest;

use reqwest::{Error, header, Client};

#[macro_use] extern crate serde_derive;
#[derive(Deserialize)]
struct Member {
    user: User
}

#[derive(Deserialize)]
struct User {
    username: String,
    discriminator: String,
    id: String,
}

#[derive(Deserialize)]
struct Channel {
    name: String,
    id: String,
    #[serde(rename = "type")]
    ty: i8,
}

#[derive(Serialize)]
struct Message {
    content: String,
}

struct LoftBot {
    client: Client,
    guild_id: String,
}

impl LoftBot {
    fn new(guild_id: String) -> Result<LoftBot, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, 
            header::HeaderValue::from_static("DiscordBot (bogodynamics.io, 1.0)"));
        let token = "Bot NTEyMDgxODQ0MzQzMTQ0NDU4.Dxp1hw.7iC-_L8jx8Mf3A8RK3K7IRFQd4w";
        headers.insert(header::AUTHORIZATION, 
            header::HeaderValue::from_static(token));
        let client = Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(LoftBot {
            client,
            guild_id
        } )
    }
    fn get_channels(&self) -> Result<Vec<Channel>, Error> {
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/channels", self.guild_id);
        let mut body: Vec<Channel> = self.client.get(url).send()?.json()?;
        body.retain(|x| x.ty == 0);
        Ok(body)
    }
    fn get_online_members(&self) -> Result<Vec<Member>, Error> {
        let url = &format!("https://discordapp.com/api/v6/guilds/{}/members", self.guild_id);
        let body: Vec<Member> = self.client.get(url).send()?.json()?;
        Ok(body)
    }
    fn create_message(&self, message: Message, channel_id: String) -> Result<(), Error> {
        let url = &format!("https://discordapp.com/api/v6/channes/{}/messages", channel_id);
        let res = self.client.post(url).form(&message).send()?.text()?;
        println!("{}", res);
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let guild_id: String = String::from("533354016818593846");
    let bot = LoftBot::new(guild_id)?;
    //let result = bot.get_channels()?;
    //println!("{}", result);

    Ok(())
}
