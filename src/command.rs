

enum NotifLocation {
    Discord,
    IRC,
}

pub struct User {
    discord_name: String,
    irc_name: String,
    notif_location_default: NotifLocation,
}

