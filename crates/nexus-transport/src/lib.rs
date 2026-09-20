use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Result;
use nexus_core::NexusMessage;

pub fn read_message(stream: &mut TcpStream) -> Result<NexusMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    let message = NexusMessage::deserialize(&body)?;
    Ok(message)
}

pub fn write_message(stream: &mut TcpStream, message: &NexusMessage) -> Result<()> {
    let bytes = message.serialize()?;
    let len = (bytes.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

pub fn bind_listener(addr: &str) -> Result<TcpListener> {
    Ok(TcpListener::bind(addr)?)
}

pub fn connect(addr: &str) -> Result<TcpStream> {
    Ok(TcpStream::connect(addr)?)
}
