use crate::event::DiscordUser;

enum NotifLocation {
    Discord,
    IRC,
}

pub struct User {
    discord_user: DiscordUser,
    irc_name: String,
    notif_location_default: NotifLocation,
}

