use gonk_tcp::{Client, Request, Server};
use std::thread;
fn main() {
    thread::spawn(|| {
        Server::run();
    });
    let mut client = Client::new();
    client.send(Request::GetElapsed);
    loop {
        client.update();
    }
}
