use std::{io::Write, net::{TcpStream, UdpSocket, Shutdown}, sync::{Arc, atomic::AtomicBool, atomic::Ordering}, thread, time::Instant};
use std::time::Duration;

use commands::PingCommand;
use crossbeam_channel::{Sender, Receiver};
use handle_factory::HandleFactory;
use network::{Message, Protocol};
use crate::{commands::Command};
use byteorder::ReadBytesExt;

mod ff_simple;
mod go;
mod network;
mod commands;
mod controller_manager;
mod handle_factory;

fn main() {
    //let mut handle_factory = HandleFactory::new();
    //let controller_handle = handle_factory.next();
    let controller_handle = 1234;
    let (command_sender, command_receiver): (Sender<Message>, Receiver<Message>) = crossbeam_channel::bounded(10);
    let (rumble_sender, rumble_receiver): (Sender<Box<dyn commands::Command>>, Receiver<Box<dyn commands::Command>>) = crossbeam_channel::bounded(10);
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let network_thread = thread::spawn({
        let should_shutdown = should_shutdown.clone();
        move || {
            let connect = || {
                loop {
                    match network::connect("192.168.15.15", controller_handle) {
                        network::ConnectResult::Bad => {
                            println!("Unable to connect :(, trying again in 2 seconds.");
                            if should_shutdown.load(Ordering::Relaxed) {
                                return None;
                            }
                            thread::sleep(Duration::from_secs(2));
                        },
                        network::ConnectResult::Good(stream) => {
                            println!("Connected to the server!");
                            return Some(stream);
                        }
                    };
                }
            };
    
            let mut connection_optional = connect();
            let timeout = Duration::from_secs(1);
            let ping_interval = Duration::from_secs(1);
            let ping = PingCommand::new();
            let mut last_ping = Instant::now();
            loop {
    
                if should_shutdown.load(Ordering::Relaxed) {
                    if let Some(connection) = &mut connection_optional {
                        network::close(&mut connection.0);
                    }
                    return;
                }
    
                match &mut connection_optional {
                    Some(connection) => {
                        if let Ok(val) = command_receiver.recv_timeout(timeout) {
                            match val {
                                Message::Terminate => return,
                                Message::UdpData(data) => {
                                    if let Err(e) = connection.1.send(data.byte_data()) {
                                        println!("Unable to send UDP data :(. Dropping and reconnecting. Error: {}", e);
                                        connection_optional = None;
                                        continue;
                                    }
                                }
                                Message::TcpData(data) => {
                                    if let Err(e) = connection.0.write_all(data.byte_data()) {
                                        println!("Unable to send TCP data :(. Dropping and reconnecting. Error: {}", e);
                                        connection_optional = None;
                                        continue;
                                    }
                                }
                            };
                        }
            
                        // move the ping to another thread for lower latency
                        let now = Instant::now();
                        if now > last_ping + ping_interval {
                            println!("Ping!");
                            match connection.0.write_all(ping.byte_data()) {
                                Err(e) => {
                                    println!("Unable to ping :(. Reconnecting... Error: {}", e);
                                    connection_optional = None;
                                    continue;
                                }
                                Ok(_) => {
                                    match connection.0.read_u8() {
                                        Ok(val) => {
                                            if val == Protocol::TcpCommandPong.into() {
                                                println!("Pong!")
                                            } else {
                                                connection_optional = None;
                                                continue;
                                            }
                                        },
                                        Err(_) => {
                                            connection_optional = None;
                                            continue;
                                        }
                                    }
                                }
                            };
                            last_ping = now;
                        }
                    },
                    None => {
                        connection_optional = connect();
                    }
                }
            }
        }
    });

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    go::go(command_sender);

    should_shutdown.store(true, Ordering::Relaxed);
    let _ = network_thread.join();
    //ff_simple::ff_simple();
}
