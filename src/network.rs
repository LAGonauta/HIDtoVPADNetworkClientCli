use std::{net::{IpAddr, SocketAddr, TcpStream, UdpSocket}, sync::{Arc, atomic::AtomicBool, atomic::Ordering}, thread::{self, JoinHandle}, time::{Duration, Instant}};
use std::io::Write;
use bytebuffer::ByteBuffer;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use flume::{Receiver, Sender};

use crate::commands::{AttachCommand, AttachData, AttachResponse, Command, PingCommand, Rumble};

// change reconnection sender to an enum: Disconnected, Reconnected
pub fn start_thread(
    wiiu_ip: IpAddr,
    command_receiver: Receiver<Message>,
    reconnection_sender: Sender<()>,
    rumble_sender: Sender<Rumble>,
    should_shutdown: Arc<AtomicBool>
) -> JoinHandle<()> {
    thread::spawn({
        move || {
            let connect = || {
                loop {
                    match connect(wiiu_ip) {
                        ConnectResult::Bad => {
                            println!("Unable to connect :(, trying again in 2 seconds.");
                            if should_shutdown.load(Ordering::Relaxed) {
                                return None;
                            }
                            thread::sleep(Duration::from_secs(2));
                        },
                        ConnectResult::Good(stream) => {
                            println!("Connected to the server!");
                            return Some(stream);
                        }
                    };
                }
            };
    
            let mut connection = connect();
            let timeout = Duration::from_secs(1);
            let ping_interval = Duration::from_secs(1);
            let ping = PingCommand::new();
            let mut last_ping = Instant::now();
            let mut udp_buffer = [0; 1400];
            let wiiu_udp_addr = SocketAddr::new(wiiu_ip, BaseProtocol::UdpPort.into());
            loop {
    
                if should_shutdown.load(Ordering::Relaxed) {
                    if let Some(connection) = &mut connection {
                        close(&mut connection.tcp);
                    }
                    return;
                }
    
                match &mut connection {
                    Some(connection_handle) => {

                        match connection_handle.udp.recv_from(&mut udp_buffer) {
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
                            Err(_e) => {}
                        }

                        if let Ok(val) = command_receiver.recv_timeout(timeout) {
                            match val {
                                Message::Terminate => return,
                                Message::UdpData(data) => {
                                    if let Err(e) = connection_handle.udp.send_to(data.byte_data(), wiiu_udp_addr) {
                                        println!("Unable to send UDP data. Dropping and reconnecting. Error: {}", e);
                                        connection = None;
                                        continue;
                                    }
                                }
                                Message::TcpData(data) => {
                                    if let Err(e) = connection_handle.tcp.write_all(data.byte_data()) {
                                        println!("Unable to send TCP data. Dropping and reconnecting. Error: {}", e);
                                        connection = None;
                                        continue;
                                    }
                                }
                                Message::Attach(val) => {
                                    let _r = val.response.send(attach_controller(val.handle, &mut connection_handle.tcp));
                                }
                            };
                        }
            
                        // move the ping to another thread for lower latency
                        let now = Instant::now();
                        if now > last_ping + ping_interval {
                            //println!("Ping!");
                            match connection_handle.tcp.write_all(ping.byte_data()) {
                                Err(e) => {
                                    println!("Unable to ping :(. Reconnecting... Error: {}", e);
                                    connection = None;
                                    continue;
                                }
                                Ok(_) => {
                                    match connection_handle.tcp.read_u8() {
                                        Ok(val) => {
                                            if val == TcpProtocol::TcpCommandPong.into() {
                                                //println!("Pong!")
                                            } else {
                                                connection = None;
                                                continue;
                                            }
                                        },
                                        Err(_) => {
                                            connection = None;
                                            continue;
                                        }
                                    }
                                }
                            };
                            last_ping = now;
                        }
                    },
                    None => {
                        connection = connect();
                        let _r = reconnection_sender.send(());
                    }
                }
            }
        }
    })
}

fn connect(ip: &str) -> ConnectResult {
    let addr: SocketAddr = format!("{}:{}", ip, BaseProtocol::TcpPort as i16)
        .parse().expect("Unable to parse socket address");
fn connect(wiiu_ip: IpAddr) -> ConnectResult {
    let addr: SocketAddr = SocketAddr::new(wiiu_ip, BaseProtocol::TcpPort.into());
    let tcp_stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

            match handshake(&mut stream) {
                HandshakeResult::Bad => {
                    println!("Handshake timeout");
                    return ConnectResult::Bad;
                }
                HandshakeResult::Good => {}
            };

            stream
        }
        Err(_) => {
            println!("Couldn't connect to server...");
            return ConnectResult::Bad;
        }
    };

    let udp_stream = match udp_bind() {
        Some(val) => val,
        None => return ConnectResult::Bad
    };

    ConnectResult::Good(Connection { tcp: tcp_stream, udp: udp_stream })
}

fn udp_bind() -> Option<UdpSocket> {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", BaseProtocol::UdpServerPort as i16)) {
        Ok(val) => val,
        Err(e) => {
            println!("Unable to bind UDP: {}", e);
            return None
        }
    };

    let _ = socket.set_nonblocking(true);

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

#[derive(Copy, Clone)]
enum HandshakeResult {
    Bad,
    Good
}

pub enum ConnectResult {
    Bad,
    Good(Connection)
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

pub enum Message {
    Terminate,
    TcpData(Box<dyn Command>),
    UdpData(Box<dyn Command>),
    Attach(AttachData)
}

pub struct Connection {
    pub tcp: TcpStream,
    pub udp: UdpSocket
}