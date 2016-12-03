/*
 * Code in this file has been taken and modifed from
 * https://github.com/tmerr/i3ipc-rs
 */

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use std::io::{Read, Write, self};
use unix_socket::UnixStream;

pub const R3_UNIX_SOCK: &'static str = "/tmp/rubar.sock";

pub const RESERVED: u32 = 20;
pub const REPLY: u32 = 21;
pub const UNPARK: u32 = 22;
pub const WEBPACK: u32 = 23;

pub trait R3Funcs {
    fn send_i3_message(&mut self, u32, &str) -> io::Result<()>;
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)>;
}

impl R3Funcs for UnixStream {

    fn send_i3_message(&mut self, message_type: u32, payload: &str) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(14 + payload.len());
        bytes.extend("i3-ipc".bytes());                         // 6 bytes
        bytes.write_u32::<LittleEndian>(payload.len() as u32)?; // 4 bytes
        bytes.write_u32::<LittleEndian>(message_type)?;         // 4 bytes
        bytes.extend(payload.bytes());                          // payload.len() bytes
        self.write_all(&bytes[..])
    }

    /// returns a tuple of (message type, payload)
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)> {
        let mut magic_data = [0_u8; 6];
        if let Err(e) = self.read_exact(&mut magic_data) {
            return Err(e);
        }
        let magic_string = String::from_utf8_lossy(&magic_data);
        if magic_string != "i3-ipc" {
            let error_text = format!(
                "unexpected magic string: expected 'i3-ipc' but got {}",
                magic_string
            );
            return Err(io::Error::new(io::ErrorKind::Other, error_text));
        }
        let payload_len = self.read_u32::<LittleEndian>()?;
        let message_type = self.read_u32::<LittleEndian>()?;
        let mut payload_data = vec![0_u8 ; payload_len as usize];
        if let Err(e) = self.read_exact(&mut payload_data[..]) {
            return Err(e);
        };
        let payload_string = String::from_utf8_lossy(&payload_data).into_owned();
        Ok((message_type, payload_string))
    }
}
