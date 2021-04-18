use flume::Sender;
use gilrs::{GamepadId, ff::Effect};

use crate::commands::Command;

pub struct Controller {
    pub id: GamepadId,
    pub handle: i32,
    pub device_slot: i16,
    pub pad_slot: i8,
    pub effect: Option<Effect>
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

pub enum PingResponse {
    Pong,
    Disconnect
}

pub struct AttachData {
    pub handle: i32,
    pub response: Sender<Option<AttachResponse>>
}

pub struct DettachData {
    pub handle: i32
}

pub struct AttachResponse {
    pub device_slot: i16,
    pub pad_slot: i8
}

pub enum Rumble {
    Start(i32),
    Stop(i32)
}