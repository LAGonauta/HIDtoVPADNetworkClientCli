use std::{sync::{Arc, atomic::AtomicBool, atomic::Ordering}};

use commands::Rumble;
use flume::{Sender, Receiver};
use network::{BaseProtocol, Message};

use std::net::IpAddr;
use clap::{App, Arg};

mod go;
mod network;
mod commands;
mod controller_manager;
mod handle_factory;

fn main() {
    let matches = App::new("HIDtoVPAD client written in Rust")
        .version("1.0")
        .author("LAGonauta")
        .about("Connects HIDtoVPAD on your WiiU using little memory")
        .arg(Arg::with_name("polling-rate")
            .short("pr")
            .long("polling-rate")
            .value_name("POLLING_RATE")
            .help("Sets the polling rate")
            .default_value("250")
            .takes_value(true))
        .arg(Arg::with_name("IP")
            .help("Sets the ip to connect to")
            .required(true))
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .get_matches();



    let (command_sender, command_receiver): (Sender<Message>, Receiver<Message>) = flume::bounded(2);
    let (reconection_notifier_sender, reconection_notifier_receiver): (Sender<()>, Receiver<()>) = flume::unbounded();
    let (rumble_sender, rumble_receiver): (Sender<Rumble>, Receiver<Rumble>) = flume::unbounded();
    let should_shutdown = Arc::new(AtomicBool::new(false));
    let addr: IpAddr = "192.168.15.15".parse().unwrap();
    let polling_rate: u32 = 250; // Hz

    let network_thread = network::start_thread(addr, command_receiver, reconection_notifier_sender, rumble_sender, should_shutdown.clone());

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    let go_thread = std::thread::spawn({
        let should_shutdown = should_shutdown.clone();
        move || {
            go::go(polling_rate, command_sender, reconection_notifier_receiver, rumble_receiver, should_shutdown);
        }
    });

    ctrlc::set_handler({
        let should_shutdown = should_shutdown.clone();
        move || {
            should_shutdown.store(true, Ordering::Relaxed);
        }
    })
    .expect("Error setting Ctrl-C handler");

    println!("Press enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());

    should_shutdown.store(true, Ordering::Relaxed);
    let _ = network_thread.join();
    let _ = go_thread.join();
}
