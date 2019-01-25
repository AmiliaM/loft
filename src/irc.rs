use crate::event;

use futures::Future;
use futures::stream::Stream;
use futures::sync::mpsc::Receiver;

use xirc::Connection;

pub use xirc::Event;

pub fn connect(host: std::net::SocketAddr, nick: &str, user: &str, sender: futures::sync::mpsc::Sender<event::Event>, receiver: Receiver<xirc::Command>) {
    let mut builder = Connection::builder(host, nick);
    builder.user(user);
    let connection = builder.build().expect("mlem");
    tokio::spawn(
        connection
            .connect::<String>("irc.freenode.net".to_string())
            .map(|s| {
                let (writer, reader) = s.split();
                
                tokio::spawn(reader.map(|event| event::Event::IRCEvent(event)).forward(sender).map(|_| ()).map_err(|e| println!("IRC read err: {}", e)));

                tokio::spawn(
                    receiver.map_err(|_| failure::err_msg("IRC stream receive error"))
                    .forward(writer)
                    .map(|_| ())
                    .map_err(|e| println!("IRC write error: {}", e))
                );
            }).map_err(|e| println!("IRC connect err: {}", e))
    );

}