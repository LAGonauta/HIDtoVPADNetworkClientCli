use std::{io::Write, net::{TcpStream, UdpSocket, Shutdown}, thread, time::Instant};
use std::time::Duration;

use commands::PingCommand;
use crossbeam_channel::{Sender, Receiver};
use handle_factory::HandleFactory;
use network::Protocol;
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
    let (command_sender, command_receiver): (Sender<Box<dyn commands::Command>>, Receiver<Box<dyn commands::Command>>) = crossbeam_channel::bounded(10);
    let (shutdown_sender, shutdown_receiver): (Sender<()>, Receiver<()>) = crossbeam_channel::unbounded();

    let network_thread = thread::spawn(move || {
        let connect = || {
            loop {
                match network::connect("192.168.15.15", controller_handle) {
                    network::ConnectResult::Bad => {
                        println!("Unable to connect :(, trying again in 2 seconds.");
                        thread::sleep(Duration::from_secs(2));
                    },
                    network::ConnectResult::Good(stream) => {
                        println!("Connected to the server!");
                        return stream;
                    }
                };
            }
        };

        let mut connection = connect();
        let timeout = Duration::from_secs(1);
        let ping_interval = Duration::from_secs(1);
        let ping = PingCommand::new();
        let mut last_ping = Instant::now();
        loop {

            if let Ok(val) = command_receiver.recv_timeout(timeout) {
                if let Err(e) = connection.1.send(val.byte_data()) {
                    println!("Unable to send data :(. Dropping and reconnecting. Error: {}", e);
                    connection.1 = UdpSocket::bind("127.0.0.1:12345").unwrap();
                    connection = connect();
                }
            }

            // move the ping to another thread for lower latency
            if last_ping.elapsed() > ping_interval {
                //println!("Ping!");
                match connection.0.write_all(ping.byte_data()) {
                    Err(e) => {
                        println!("Unable to ping :(. Reconnecting... Error: {}", e);
                        connection.1 = UdpSocket::bind("127.0.0.1:12345").unwrap();
                        connection = connect();
                    }
                    Ok(_) => {
                        match connection.0.read_u8() {
                            Ok(val) => {
                                if val == Protocol::TcpCommandPong.into() {
                                    //println!("Pong!")
                                } else {
                                    connection.1 = UdpSocket::bind("127.0.0.1:12345").unwrap();
                                    connection = connect();
                                }
                            },
                            Err(_) => {
                                connection.1 = UdpSocket::bind("127.0.0.1:12345").unwrap();
                                connection = connect();
                            }
                        }
                    }
                };
                last_ping = Instant::now();
            }

            if shutdown_receiver.try_recv().is_ok() {
                network::close(&mut connection.0);
                return;
            }
        }
    });

    // 1. build controllers with gamepadId and handle

    // 2. connect

    // 3. send data

    go::go(command_sender);

    let _ = shutdown_sender.send(());
    let _ = network_thread.join();
    //ff::ff();
    //ff_simple::ff_simple();
}

