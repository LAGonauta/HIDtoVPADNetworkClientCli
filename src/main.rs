mod ff;
mod ff_simple;
mod go;
mod network;
mod commands;

fn main() {
    let mut stream = match network::connect("192.168.15.15") {
        network::ConnectResult::Bad => {
            println!("Unable to connect :(");
            return;
        },
        network::ConnectResult::Good(stream) => stream
    };

    network::close(&mut stream);

    //go::go();
    //ff::ff();
    //ff_simple::ff_simple();
}

