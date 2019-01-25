#[derive(Deserialize)]
pub struct Member {
    pub user: DiscordUser
} 

#[derive(Deserialize)]
pub struct DiscordUser {
    pub username: String,
    pub discriminator: String,
    pub id: String,
}

pub enum NotifLocation {
    Discord,
    IRC,
}

pub enum UserVar {
    Notif,
    Nick,
    FavFood,
    None,
}

pub struct User {
    pub discord_user: DiscordUser,
    pub irc_name: Option<String>,
    pub notif_location: Option<NotifLocation>,
    pub favorite_food: Option<String>
}

impl User {
    pub fn from_discord(user: DiscordUser) -> User {
        User {
            discord_user: user,
            irc_name: None, 
            notif_location: None, 
            favorite_food: None
        }
    }
}