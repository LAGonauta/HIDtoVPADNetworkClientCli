use handle_factory::HandleFactory;

mod ff;
mod ff_simple;
mod go;
mod network;
mod commands;
mod controller_manager;
mod handle_factory;

fn main() {
    let mut handle_factory = HandleFactory::new();
    let controller_handle = handle_factory.next();

    let mut connection = match network::connect("192.168.15.15", controller_handle) {
        network::ConnectResult::Bad => {
            println!("Unable to connect :(");
            return;
        },
        network::ConnectResult::Good(stream) => stream
    };

    network::close(&mut connection);

    //go::go();
    //ff::ff();
    //ff_simple::ff_simple();
}

