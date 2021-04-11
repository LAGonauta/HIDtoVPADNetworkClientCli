use std::{net::{SocketAddr, TcpStream}, time::Duration};
use std::io::{Write, Read};

static PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::Version3;

pub fn connect(_ip: &str) -> ConnectResult {
    let addr = SocketAddr::from(([192,168,15,15], Protocol::TcpPort.into()));
    match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(mut stream) => {
            println!("Connected to the server!");
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

            match handshake(&mut stream) {
                HandshakeResult::Bad => println!("Handshake timeout"),
                HandshakeResult::Good => {}
            };

            ConnectResult::Good(stream)
        }
        Err(_) => {
            println!("Couldn't connect to server...");
            ConnectResult::Bad
        }
    }
}

fn handshake(stream: &mut TcpStream) -> HandshakeResult {
    let mut buffer = [0; 1];
    match stream.read(&mut buffer) {
        Ok(count) => {
            if count == 0 {
                return HandshakeResult::Bad;
            }
        },
        Err(e) => {
            return HandshakeResult::Bad;
        }
    };

    let server_protocol_version: ProtocolVersion = buffer[0].into();
    println!("Server Version: {:?}", server_protocol_version);

    if server_protocol_version != ProtocolVersion::Version3 {
        println!("We only support {:?}, aborting. Current: {:?}", ProtocolVersion::Version3, server_protocol_version);
        return HandshakeResult::Bad;
    }

    buffer[0] = server_protocol_version.into();
    match stream.write(&buffer) {
        Ok(count) => {
            if count == 0 {
                return HandshakeResult::Bad;
            }
        },
        Err(e) => {
            return HandshakeResult::Bad;
        }
    }

    match stream.read(&mut buffer) {
        Ok(count) => {
            if count == 0 {
                return HandshakeResult::Bad;
            }
        },
        Err(e) => {
            return HandshakeResult::Bad;
        }
    };

    let final_response: ProtocolVersion = buffer[0].into();
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
pub enum Protocol {
    Version = ProtocolVersion::Version3 as isize,

    TcpPort = 8112,
    UdpPort = 8113,
    UdpClientPort = 8114,

    TcpCommandAttach = 0x01,
    TcpCommandDetach = 0x02,
    TcpCommandPing = 0xF0,

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