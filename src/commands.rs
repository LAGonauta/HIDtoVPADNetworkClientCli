use std::{fmt, io::{Read, Write}};

use bytebuffer::ByteBuffer;

use crate::network::Protocol;

pub trait Command {
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
        buffer.write_u8(Protocol::TcpCommandAttach.into());
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
        buffer.write_u8(Protocol::TcpCommandDetach.into());
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
    handle: i32,
    sender: i32,
    data: Vec<u8>
}

impl WriteCommand {
    pub fn new(handle: i32, sender: i32, data: Vec<u8>) -> WriteCommand {
        WriteCommand {
            handle,
            sender,
            data
        }
    }
}

impl Command for WriteCommand {
    fn data(&self) -> String {
        format!("WriteCommand [handle={}, sender={}]", self.handle, self.sender)
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
            data: vec![Protocol::TcpCommandPing.into()]
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