use std::io::Write;

use bytebuffer::ByteBuffer;

use crate::models::{Controller, TcpProtocol, UdpProtocol};

pub trait Command : Send {
    fn data(&self) -> String;
    fn byte_data(&self) -> &Vec<u8>;
}

pub struct AttachCommand {
    handle: i32,
    vid: i16,
    pid: i16,
    sender: i32,
    data: Vec<u8>
}

impl AttachCommand {
    pub fn new(handle: i32, vid: i16, pid: i16, sender: i32) -> AttachCommand {
        let mut buffer = ByteBuffer::new();
        buffer.write_u8(TcpProtocol::TcpCommandAttach.into());
        buffer.write_i32(handle);
        buffer.write_i16(vid);
        buffer.write_i16(pid);

        AttachCommand {
            handle,
            vid,
            pid,
            sender,
            data: buffer.to_bytes()
        }
    }
}

impl Command for AttachCommand {
    fn data(&self) -> String {
        format!("AttachCommand [vid={}, pid={}, handle={}, sender={}]", self.vid, self.pid, self.handle, self.sender)
    }

    fn byte_data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub struct DetachCommand {
    handle: i32,
    sender: i32,
    data: Vec<u8>
}

impl DetachCommand {
    pub fn new(handle: i32, sender: i32) -> DetachCommand {
        let mut buffer = ByteBuffer::new();
        buffer.write_u8(TcpProtocol::TcpCommandDetach.into());
        buffer.write_i32(handle);

        DetachCommand {
            handle,
            sender,
            data: buffer.to_bytes()
        }
    }
}

impl Command for DetachCommand {
    fn data(&self) -> String {
        format!("DetachCommand [handle={}, sender={}]", self.handle, self.sender)
    }

    fn byte_data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub struct WriteCommand {
    sender: i32,
    data: Vec<u8>
}

impl WriteCommand {
    pub fn new(controllers: &Vec<(&Controller, Vec<u8>)>, sender: i32) -> WriteCommand {
        let mut buffer = ByteBuffer::new();
        buffer.write_u8(UdpProtocol::UdpCommandData.into());
        if controllers.len() > 0xFF {
            println!("Too many controllers!");
        }
        buffer.write_u8(controllers.len() as u8);
        for controller in controllers {
            buffer.write_i32(controller.0.handle);
            buffer.write_i16(controller.0.device_slot);
            buffer.write_i8(controller.0.pad_slot);
            buffer.write_i8((controller.1.len() & 0xFF) as i8);
            buffer.write_all(&controller.1).unwrap();
        }

        WriteCommand {
            sender,
            data: buffer.to_bytes()
        }
    }
}

impl Command for WriteCommand {
    fn data(&self) -> String {
        format!("WriteCommand [sender={}]", self.sender)
    }

    fn byte_data(&self) -> &Vec<u8> {
        &self.data
    }
}

pub struct PingCommand {
    data: Vec<u8>
}

impl PingCommand {
    pub fn new() -> PingCommand {
        PingCommand {
            data: vec![TcpProtocol::TcpCommandPing.into()]
        }
    }
}

impl Command for PingCommand {
    fn data(&self) -> String {
        "PingCommand []".to_owned()
    }

    fn byte_data(&self) -> &Vec<u8> {
        &self.data
    }
}