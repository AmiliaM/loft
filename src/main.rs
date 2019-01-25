#[macro_use] extern crate serde_derive;

mod bot;
mod event;
mod command;
mod user;

use self::bot::LoftBot;
use futures::Future;

fn main() -> Result<(), failure::Error> {
    let guild_id = String::from("533354016818593846");
    tokio::run(LoftBot::run(guild_id).map_err(|x| println!("error: {}", x)));
    Ok(())
}
