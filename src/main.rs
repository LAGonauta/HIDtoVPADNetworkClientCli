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
    //let controller_handle = handle_factory.next();
    let controller_handle = 1234;

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    let mut connection = match network::connect("192.168.15.15", controller_handle) {
        network::ConnectResult::Bad => {
            println!("Unable to connect :(");
            return;
        },
        network::ConnectResult::Good(stream) => stream
    };

    go::go(&mut connection.1);

    network::close(&mut connection.0);
    //ff::ff();
    //ff_simple::ff_simple();
}

