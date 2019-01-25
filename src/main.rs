#[macro_use] extern crate serde_derive;

mod bot;
mod event;
mod command;
mod user;
mod irc;

use self::bot::LoftBot;
use futures::Future;
use std::collections::HashMap;

fn main() -> Result<(), failure::Error> {
    let config = std::fs::read_to_string("config.txt")?;
    let lines: Vec<_> = config.lines().collect();
    let mut args: HashMap<String, String> = HashMap::new();
    for line in lines {
        let a: Vec<_> = line.split("=").map(|s| s.trim()).collect();
        args.insert(a[0].to_string(), a[1].to_string());
    }
    tokio::run(LoftBot::run(args).map_err(|x| println!("error: {}", x)));
    Ok(())
}
