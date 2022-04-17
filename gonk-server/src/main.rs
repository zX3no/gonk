use gonk_server::Server;

fn main() {
    //should take care of panics on different threads
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    Server::run();
}
