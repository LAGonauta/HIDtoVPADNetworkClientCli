use std::{net::{IpAddr, SocketAddr, TcpStream, UdpSocket}, sync::{Arc, atomic::AtomicBool, atomic::Ordering}, thread::{self, JoinHandle}, time::{Duration, Instant}};
use std::io::Write;
use bytebuffer::ByteBuffer;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use flume::{Receiver, Sender};

use crate::commands::{AttachCommand, AttachData, AttachResponse, Command, DettachData, PingCommand, Rumble};

// change reconnection sender to an enum: Disconnected, Reconnected
pub fn start_thread(
    wiiu_ip: IpAddr,
    tcp_command_sender: Sender<TcpMessage>,
    tcp_command_receiver: Receiver<TcpMessage>,
    udp_command_receiver: Receiver<UdpMessage>,
    reconnection_sender: Sender<()>,
    rumble_sender: Sender<Rumble>,
    should_shutdown: Arc<AtomicBool>
) -> JoinHandle<()> {
    thread::spawn({
        move || {
            let timeout = Duration::from_secs(1);
            let ping_interval = Duration::from_secs(1);
            let mut last_ping = Instant::now();

            let tcp_thread = start_tcp_thread(tcp_command_receiver.clone(), wiiu_ip, should_shutdown.clone());

            let udp_thread = start_command_send_thread(udp_command_receiver, wiiu_ip, should_shutdown.clone());

            let rumble_thread = start_rumble_thread(rumble_sender, wiiu_ip, should_shutdown.clone());

            loop {
                if should_shutdown.load(Ordering::Relaxed) {
                    break;
                }
                // send ping commands, rate limited
                break;
            }

            let _ = udp_thread.join();
            let _ = rumble_thread.join();
            let _ = tcp_thread.join();
        }
    })
}

fn start_tcp_thread(
    receiver: Receiver<TcpMessage>,
    wiiu_ip: IpAddr,
    should_shutdown: Arc<AtomicBool>
) -> JoinHandle<()> {
    thread::spawn(move || {
        let timeout = Duration::from_secs(1);
        let ping = PingCommand::new();
        let mut stream = TcpConnectionResult::Bad;
        loop {
            if should_shutdown.load(Ordering::Relaxed) {
                match stream {
                    TcpConnectionResult::Bad => {}
                    TcpConnectionResult::Good(ref mut stream) => {
                        close(stream);
                    }
                }
                return;
            }

            match stream {
                TcpConnectionResult::Good(ref mut stream) => {
                    match receiver.recv_timeout(timeout) {
                        Ok(action) => {
                            match action {
                                TcpMessage::Ping(sender) => {
                                    //println!("Ping!");
                                    match stream.write_all(ping.byte_data()) {
                                        Err(e) => {
                                            println!("Unable to ping :(. Reconnecting... Error: {}", e);
                                            let _ = sender.send_timeout(PingResponse::Disconnect, timeout);
                                        }
                                        Ok(_) => {
                                            match stream.read_u8() {
                                                Ok(val) => {
                                                    if val == TcpProtocol::TcpCommandPong.into() {
                                                        //println!("Pong!")
                                                        let _ = sender.send_timeout(PingResponse::Pong, timeout);
                                                    } else {
                                                        let _ = sender.send_timeout(PingResponse::Disconnect, timeout);
                                                    }
                                                },
                                                Err(_) => {
                                                    let _ = sender.send_timeout(PingResponse::Disconnect, timeout);
                                                }
                                            }
                                        }
                                    };
                                }
                                TcpMessage::Attach(attach_data) => {
                                    let _r =
                                        attach_data.response.send(attach_controller(attach_data.handle, stream));
                                }
                                TcpMessage::Dettach(_) => {}
                            }
        
                        }
                        Err(_) => {}
                    };
                },
                TcpConnectionResult::Bad => {
                    // everyone should disconnect and wait for reconnection command
                    stream = tcp_connect(wiiu_ip);
                    match stream {
                        TcpConnectionResult::Bad => {
                            println!("Unable to connect, waiting 2 seconds to try again");
                            thread::sleep(Duration::from_secs(2));
                        },
                        TcpConnectionResult::Good(_) => {
                            // everyone can reconnect
                        }
                    }
                }
            }
        }
    })
}

fn start_command_send_thread(
    command_receiver: Receiver<UdpMessage>,
    wiiu_ip: IpAddr,
    should_shutdown: Arc<AtomicBool>
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut udp_socket: Option<UdpSocket> = None;
        let receive_timeout = Duration::from_secs(1);
        loop {
            if should_shutdown.load(Ordering::Relaxed) {
                return;
            }

            match udp_socket {
                Some(ref socket) => {
                    if let Ok(val) = command_receiver.recv_timeout(receive_timeout) {
                        match val {
                            UdpMessage::UdpData(data) => {
                                if let Err(e) = socket.send(data.byte_data()) {
                                    println!("Unable to send UDP data. Dropping and reconnecting. Error: {}", e);
                                    udp_socket = None;
                                    continue;
                                }
                            }
                        };
                    }
                },
                None => {
                    udp_socket = udp_bind(BaseProtocol::UdpPort.into());
                    match udp_socket {
                        Some(ref socket) => {
                            match socket.connect(SocketAddr::new(wiiu_ip, BaseProtocol::UdpPort.into())) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Unable to connect to send controller commands, trying again in 1 second: {}", e);
                                    thread::sleep(Duration::from_secs(1));
                                }
                            };
                        },
                        None => {
                            println!("Unable to connect to send controller commands, trying again in 1 second");
                            thread::sleep(Duration::from_secs(1));
                        }
                    }
                }
            }
        }
    })
}

fn start_rumble_thread(
    rumble_sender: Sender<Rumble>,
    wiiu_ip: IpAddr,
    should_shutdown: Arc<AtomicBool>
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut udp_socket: Option<UdpSocket> = None;
        let mut udp_buffer = [0; 1400];
        loop {
            // add rate limiter?
            if should_shutdown.load(Ordering::Relaxed) {
                return;
            }

            match udp_socket {
                Some(ref socket) => {
                    match socket.recv_from(&mut udp_buffer) {
                        Ok((count, addr)) => {
                            if count >= 6 && addr.ip() == wiiu_ip {
                                let mut data = ByteBuffer::from_bytes(&udp_buffer);

                                if data.read_u8() == UdpProtocol::UdpCommandRumble.into() {
                                    let handle = data.read_i32();
                                    if data.read_u8() == UdpProtocol::UdpCommandRumble.into() {
                                        let _ = rumble_sender.send(Rumble::Start(handle));
                                    } else {
                                        let _ = rumble_sender.send(Rumble::Stop(handle));
                                    }
                                }
    
                                udp_buffer.fill(0);
                            }
                        }
                        Err(_e) => {
                            // reconnect? what if it times out?
                        }
                    }
                },
                None => {
                    udp_socket = udp_bind(BaseProtocol::UdpServerPort.into());
                    if udp_socket.is_none() {
                        println!("Unable to open UDP server for rumble, trying again in 1 second.");
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        }
    })
}

fn tcp_connect(wiiu_ip: IpAddr) -> TcpConnectionResult {
    let addr: SocketAddr = SocketAddr::new(wiiu_ip, BaseProtocol::TcpPort.into());
    match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(mut stream) => {
            //let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            //let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

            match handshake(&mut stream) {
                HandshakeResult::Bad => println!("Handshake timeout"),
                HandshakeResult::Good => return TcpConnectionResult::Good(stream)
            };
        }
        Err(_) => {
            println!("Couldn't connect to server...");
        }
    };

    TcpConnectionResult::Bad
}

fn udp_bind(port: i16) -> Option<UdpSocket> {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
        Ok(val) => val,
        Err(e) => {
            println!("Unable to bind UDP: {}", e);
            return None
        }
    };

    let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = socket.set_write_timeout(Some(Duration::from_secs(1)));

    return Some(socket);
}

fn attach_controller(controller_handle: i32, stream: &mut TcpStream) -> Option<AttachResponse> {
    let command = AttachCommand::new(controller_handle, 0x7331, 0x1337, 1);
    send_attach(&command, stream)
}

fn handshake(stream: &mut TcpStream) -> HandshakeResult {
    let server_protocol_version = match stream.read_u8() {
        Ok(val) => val,
        Err(_e) => return HandshakeResult::Bad
    };
    
    println!("Server Version: {:?}", server_protocol_version);

    if server_protocol_version != ProtocolVersion::Version3.into() {
        println!("We only support {:?}, aborting. Current: {:?}", ProtocolVersion::Version3, server_protocol_version);
        return HandshakeResult::Bad;
    }

    match stream.write_u8(server_protocol_version) {
        Ok(_) => {},
        Err(_e) => return HandshakeResult::Bad
    }

    let final_response: ProtocolVersion = match stream.read_u8() {
        Ok(val) => val,
        Err(_e) => return HandshakeResult::Bad
    }.into();

    match final_response {
        ProtocolVersion::Unknown => {
            println!("Something stranged happend while connecting. Try to use the newest version of HIDtoVPAD and this network client.");
            return HandshakeResult::Bad;
        },
        ProtocolVersion::Abort => {
            println!("The HIDtoVPAD version is not supported. Try to use the newest version of HIDtoVPAD and this network client.");
            return HandshakeResult::Bad;
        },
        _ => {}
    }

    return HandshakeResult::Good;
}

fn send_attach(command: &AttachCommand, stream: &mut TcpStream) -> Option<AttachResponse> {
    match stream.write_all(command.byte_data()) {
        Ok(_) => {}
        Err(_) => return None
    }

    let config_found = match stream.read_u8() {
        Ok(val) => val,
        Err(_) => return None
    };
    if config_found == 0 {
        println!("Failed to get byte.");
        return None;
    } else if config_found == TcpProtocol::TcpCommandAttachConfigNotFound.into() {
        println!("No config found for this device.");
    } else if config_found == TcpProtocol::TcpCommandAttachConfigFound.into() {
        println!("Config found for this device.");
    } else {
        println!("Should not get this far :(");
    }

    let user_data_okay = match stream.read_u8() {
        Ok(val) => val,
        Err(_) => return None
    };
    if user_data_okay == 0 {
        println!("Failed to get byte.");
        return None;
    } else if user_data_okay == TcpProtocol::TcpCommandAttachUserdataBad.into() {
        println!("Bad user data.");
    } else if user_data_okay == TcpProtocol::TcpCommandAttachUserdataOkay.into() {
        println!("User data OK.");
    } else {
        println!("Should not get this far :(");
    }

    let device_slot = match stream.read_i16::<NetworkEndian>() {
        Ok(val) => val,
        Err(_) => return None
    };

    let padslot = match stream.read_i8() {
        Ok(val) => val,
        Err(_) => return None
    };

    if device_slot < 0 || padslot < 0 {
        println!("Receiving data after sending a attach failed for device ({}) failed. We need to disconnect =(", 0);
        return None;
    }

    return Some(AttachResponse { device_slot, pad_slot: padslot });
}

fn close(stream: &mut TcpStream) {
    let mut buffer = [0; 1];
    buffer[0] = ProtocolVersion::Abort.into();
    let _ = match stream.write(&buffer) {
        Ok(_) => println!("Succesfully closed connection."),
        Err(e) => println!("Unable to close connection: {:?}", e)
    };
}

enum HandshakeResult {
    Bad,
    Good
}

enum TcpConnectionResult {
    Bad,
    Good(TcpStream)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum ProtocolVersion {
    Unknown = 0x00,
    Version1 = 0x12,
    Version2 = 0x13,
    Version3 = 0x14,
    Abort = 0x30
}

impl Into<u8> for ProtocolVersion {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for ProtocolVersion {
    fn from(val: u8) -> Self {
        match val {
            0x12 => ProtocolVersion::Version1,
            0x13 => ProtocolVersion::Version2,
            0x14 => ProtocolVersion::Version3,
            0x30 => ProtocolVersion::Abort,
            _ => ProtocolVersion::Unknown
        }
    }
}

#[derive(Copy, Clone)]
pub enum BaseProtocol {
    //Version = ProtocolVersion::Version3 as isize,

    TcpPort = 8112,
    UdpPort = 8113,
    UdpServerPort = 8114,
}

impl Into<i16> for BaseProtocol {
    fn into(self) -> i16 {
        self as i16
    }
}

impl Into<u16> for BaseProtocol {
    fn into(self) -> u16 {
        self as u16
    }
}

impl Into<u8> for BaseProtocol {
    fn into(self) -> u8 {
        self as u8
    }
}

pub enum TcpProtocol {
    TcpCommandAttach = 0x01,
    TcpCommandDetach = 0x02,
    TcpCommandPing = 0xF0,
    TcpCommandPong = 0xF1,

    TcpCommandAttachConfigFound = 0xE0,
    TcpCommandAttachConfigNotFound = 0xE1,
    TcpCommandAttachUserdataOkay = 0xE8,
    TcpCommandAttachUserdataBad = 0xE9
}

impl Into<u16> for TcpProtocol {
    fn into(self) -> u16 {
        self as u16
    }
}

impl Into<u8> for TcpProtocol {
    fn into(self) -> u8 {
        self as u8
    }
}

pub enum UdpProtocol {
    UdpCommandData = 0x03,
    UdpCommandRumble = 0x01
}

impl Into<u16> for UdpProtocol {
    fn into(self) -> u16 {
        self as u16
    }
}

impl Into<u8> for UdpProtocol {
    fn into(self) -> u8 {
        self as u8
    }
}

pub enum TcpMessage {
    Attach(AttachData),
    Dettach(DettachData),
    Ping(Sender<PingResponse>)
}

pub enum UdpMessage {
    UdpData(Box<dyn Command>)
}

enum PingResponse {
    Pong,
    Disconnect
}