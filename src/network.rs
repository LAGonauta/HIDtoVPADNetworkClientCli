use std::{net::{SocketAddr, TcpStream, UdpSocket}, time::Duration};
use std::io::Write;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

use crate::commands::{AttachCommand, Command};

static PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::Version3;

pub fn connect(ip: &str, controller_handle: i32) -> ConnectResult {
    let addr: SocketAddr = format!("{}:{}", ip, Protocol::TcpPort as i16)
        .parse().expect("Unable to parse socket address");
    let mut tcp_stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
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

    let udp_stream = match udp_connect(ip) {
        Some(val) => val,
        None => return ConnectResult::Bad
    };

    if !attach_controller(controller_handle, &mut tcp_stream) {
        println!("Unable to attach");
    }
    ConnectResult::Good((tcp_stream, udp_stream))
}

fn udp_connect(ip: &str) -> Option<UdpSocket> {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", Protocol::UdpClientPort as i16)) {
        Ok(val) => val,
        Err(e) => {
            println!("Unable to bind UDP: {}", e);
            return None
        }
    };

    let addr = format!("{}:{}", ip, Protocol::UdpPort as i16);
    match socket.connect(addr) {
        Ok(_) => {},
        Err(e) => {
            println!("Unable to connect UDP: {}", e);
            return None
        }
    }

    return Some(socket);
}

fn attach_controller(controller_handle: i32, stream: &mut TcpStream) -> bool {
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

fn send_attach(command: &AttachCommand, stream: &mut TcpStream) -> bool {
    match stream.write_all(command.byte_data()) {
        Ok(_) => {}
        Err(_) => return false
    }

    let config_found = match stream.read_u8() {
        Ok(val) => val,
        Err(_) => return false
    };
    if config_found == 0 {
        println!("Failed to get byte.");
        return false;
    } else if config_found == Protocol::TcpCommandAttachConfigNotFound.into() {
        println!("No config found for this device.");
    } else if config_found == Protocol::TcpCommandAttachConfigFound.into() {
        println!("Config found for this device.");
    } else {
        println!("Should not get this far :(");
    }

    let user_data_okay = match stream.read_u8() {
        Ok(val) => val,
        Err(_) => return false
    };
    if user_data_okay == 0 {
        println!("Failed to get byte.");
        return  false;
    } else if user_data_okay == Protocol::TcpCommandAttachUserdataBad.into() {
        println!("Bad user data.");
    } else if user_data_okay == Protocol::TcpCommandAttachUserdataOkay.into() {
        println!("User data OK.");
    } else {
        println!("Should not get this far :(");
    }

    let device_slot = match stream.read_i16::<NetworkEndian>() {
        Ok(val) => val,
        Err(_) => return false
    };

    let padslot = match stream.read_i8() {
        Ok(val) => val,
        Err(_) => return false
    };

    if device_slot < 0 || padslot < 0 {
        println!("Recieving data after sending a attach failed for device ({}) failed. We need to disconnect =(", 0);// command.getSender()
        return false;
    }

    return true;
}

pub fn close(stream: &mut TcpStream) {
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
    Good((TcpStream, UdpSocket))
    //Good(TcpStream)
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
pub enum Protocol {
    Version = ProtocolVersion::Version3 as isize,

    TcpPort = 8112,
    UdpPort = 8113,
    UdpClientPort = 8114,

    TcpCommandAttach = 0x01,
    TcpCommandDetach = 0x02,
    TcpCommandPing = 0xF0,
    TcpCommandPong = 0xF1,

    TcpCommandAttachConfigFound = 0xE0,
    TcpCommandAttachConfigNotFound = 0xE1,
    TcpCommandAttachUserdataOkay = 0xE8,
    TcpCommandAttachUserdataBad = 0xE9,

    UdpCommandData = 0x03
}

impl Into<u16> for Protocol {
    fn into(self) -> u16 {
        self as u16
    }
}

impl Into<u8> for Protocol {
    fn into(self) -> u8 {
        self as u8
    }
}

pub enum Message {
    Terminate,
    TcpData(Box<dyn Command>),
    UdpData(Box<dyn Command>)
}