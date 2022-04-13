use std::thread;

use gonk_tcp::{Client, Server};

fn main() {
    thread::spawn(|| Server::new().run());
    let _client = Client::new();
    // loop {
    //     client.update();
    // }
    thread::park();
}
