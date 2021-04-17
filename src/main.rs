use std::{sync::{Arc, atomic::AtomicBool, atomic::Ordering}};

use commands::Rumble;
use flume::{Sender, Receiver};

use std::net::IpAddr;

mod go;
mod network;
mod commands;
mod controller_manager;
mod handle_factory;

fn main() {
    let (tcp_command_sender, tcp_command_receiver) = flume::unbounded();
    let (udp_command_sender, udp_command_receiver) = flume::bounded(0);

    let (reconection_notifier_sender, reconection_notifier_receiver): (Sender<()>, Receiver<()>) = flume::unbounded(); // use BUS

    let (rumble_sender, rumble_receiver) = flume::bounded(0);

    let should_shutdown = Arc::new(AtomicBool::new(false)); // use BUS?

    let addr: IpAddr = "192.168.15.15".parse().unwrap();

    let polling_rate: u32 = 250; // Hz

    let network_thread = network::start_thread(
        addr,
        tcp_command_sender.clone(),
        tcp_command_receiver,
        udp_command_receiver.clone(),
        reconection_notifier_sender,
        rumble_sender,
        should_shutdown.clone());

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    let go_thread = std::thread::spawn({
        let should_shutdown = should_shutdown.clone();
        move || {
            go::go(polling_rate,
                tcp_command_sender,
                udp_command_sender,
                reconection_notifier_receiver,
                rumble_receiver,
                should_shutdown
            );
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
