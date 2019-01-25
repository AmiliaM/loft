use crate::event;

use futures::Future;
use futures::stream::Stream;

use xirc::Connection;

pub use xirc::Event;

pub fn connect(host: std::net::SocketAddr, nick: &str, user: &str, sender: futures::sync::mpsc::Sender<event::Event>) {
    let mut builder = Connection::builder(host, nick);
    builder.user(user);
    let connection = builder.build().expect("mlem");
    tokio::spawn(
        connection
            .connect::<String>("irc.freenode.net".to_string())
            .and_then(|s| s.map(|event| event::Event::IRCEvent(event))
            .forward(sender)
            .map(|_| ())
            .from_err())
            .map_err(|x| println!("IRC read error: {}", x))
    );
}