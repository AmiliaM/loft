#[derive(Deserialize, Debug)]
pub struct Member {
    pub user: DiscordUser
} 

#[derive(Deserialize, Debug)]
pub struct DiscordUser {
    pub username: String,
    pub discriminator: String,
    pub id: String,
}

enum NotifLocation {
    Discord,
    IRC,
}

pub enum UserVar {
    Nicks,
    Notif,
    FavFood,
    None,
}

pub struct User {
    pub discord_user: DiscordUser,
    irc_names: Option<Vec<String>>,
    notif_location_default: Option<NotifLocation>,
    pub favorite_food: Option<String>
}

impl User {
    pub fn from_discord(user: DiscordUser) -> User {
        User {
            discord_user: user,
            irc_names: None, 
            notif_location_default: None, 
            favorite_food: None
        }
    }
}