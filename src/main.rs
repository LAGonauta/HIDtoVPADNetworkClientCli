use std::{sync::{Arc, atomic::AtomicBool, atomic::Ordering}};

use commands::Rumble;
use flume::{Sender, Receiver};
use network::Message;

//mod ff_simple;
mod go;
mod network;
mod commands;
mod controller_manager;
mod handle_factory;

fn main() {
    let (command_sender, command_receiver): (Sender<Message>, Receiver<Message>) = flume::bounded(10);
    let (reconection_notifier_sender, reconection_notifier_receiver): (Sender<()>, Receiver<()>) = flume::unbounded();
    let (rumble_sender, rumble_receiver): (Sender<Rumble>, Receiver<Rumble>) = flume::unbounded();
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let network_thread = network::start_thread("192.168.15.15", command_receiver, reconection_notifier_sender, rumble_sender, should_shutdown.clone());

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    go::go(command_sender, reconection_notifier_receiver, rumble_receiver, should_shutdown.clone());

    should_shutdown.store(true, Ordering::Relaxed);
    let _ = network_thread.join();
    //ff_simple::ff_simple();
}
