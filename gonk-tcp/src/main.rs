use gonk_tcp::Server;
fn main() {
    Server::new().run();
    // thread::spawn(|| {
    //     Server::run();
    // });
    // let mut client = Client::new();
    // // client.send(Request::GetElapsed);
    // loop {
    //     client.update();
    // }
}
