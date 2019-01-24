#[macro_use] extern crate serde_derive;

mod bot;
mod event;
mod command;
mod user;

use self::bot::LoftBot;
use futures::Future;

fn main() -> Result<(), failure::Error> {
    let guild_id = String::from("533354016818593846");
    let mut runtime = tokio::runtime::Runtime::new()?;
    match runtime.block_on(LoftBot::run(guild_id).map_err(|x| println!("{}", x))) {
        Ok(_) => println!("Clean exit"),
        Err(_) => println!("Error exit"),
    }
    println!("Runtime shutting down...");
    runtime.shutdown_now().wait().map_err(|_| failure::err_msg("shutdown fail"))?;
    Ok(())
}
