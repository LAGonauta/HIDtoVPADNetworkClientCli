use std::{sync::{Arc, atomic::Ordering}};

use atomic::Atomic;
use models::ApplicationState;

use std::net::IpAddr;

mod go;
mod network;
mod commands;
mod controller_manager;
mod models;

fn main() {
    let addr: IpAddr = "192.168.15.15".parse().unwrap();
    let polling_rate: u32 = 250; // Hz

    let (tcp_command_sender, tcp_command_receiver) = flume::unbounded();
    let (udp_command_sender, udp_command_receiver) = flume::bounded(0);

    let (reconection_notifier_sender, reconection_notifier_receiver) = flume::unbounded(); // use BUS

    let (rumble_sender, rumble_receiver) = flume::bounded(0);

    let application_state = Arc::new(Atomic::new(ApplicationState::Disconnected));

    let network_thread = network::start_thread(
        addr,
        tcp_command_sender.clone(),
        tcp_command_receiver,
        udp_command_receiver.clone(),
        reconection_notifier_sender,
        rumble_sender,
        application_state.clone());

    let go_thread = std::thread::spawn({
        let application_state = application_state.clone();
        move || {
            go::go(polling_rate,
                tcp_command_sender,
                udp_command_sender,
                reconection_notifier_receiver,
                rumble_receiver,
                application_state
            );
        }
    });

    ctrlc::set_handler({
        let application_state = application_state.clone();
        move || {
            application_state.store(ApplicationState::Exiting, Ordering::Relaxed);
            println!("### Press enter to finish ###");
        }
    })
    .expect("Error setting Ctrl-C handler");

    println!("### Press enter to exit ###");
    let _ = std::io::stdin().read_line(&mut String::new());
    println!("---> Exiting <---");

    application_state.store(ApplicationState::Exiting, Ordering::Relaxed);
    let _ = network_thread.join();
    let _ = go_thread.join();
}
