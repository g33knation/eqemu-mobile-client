use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::packets::{RoF2Packet, SessionRequest, LoginInfo};
use binrw::BinWrite;
use rand::Rng;

pub struct Client {
    stream: TcpStream,
    session_id: u32,
}

impl Client {
    /// Create a new client wrapper
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            session_id: rand::thread_rng().gen::<u32>(),
        }
    }

    /// The "Brain" of the client. Run this in a tokio::spawn.
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Client Starting. Session ID: 0x{:08X}", self.session_id);

        // 1. Send Handshake
        let handshake = SessionRequest { session_id: self.session_id };
        self.send(handshake).await?;

        // 2. Wait for Handshake Reply (OpCode 0x0001)
        let (opcode, _) = self.read_packet().await?; 
        if opcode == 0x0001 {
             println!("✅ Handshake Acknowledged.");
        } else {
             println!("⚠️ Unexpected Handshake Reply: 0x{:04X}", opcode);
        }

        // 3. Send Login
        let login = LoginInfo {
            username: self.string_to_bytes("eqemu"),
            password: self.string_to_bytes("eqemu"),
        };
        self.send(login).await?;
        println!("🔑 Credentials Sent.");

        // 4. Request Server List
        let list_req = crate::packets::ServerListRequest {};
        self.send(list_req).await?;
        println!("📂 Server List Requested.");

        // 5. Keep Alive / Read Loop
        loop {
            let (opcode, payload) = self.read_packet().await?;
            match opcode {
                0x0004 | 0x0018 | 0x0019 => {
                    println!("🎉 JACKPOT! Server List Received (OpCode 0x{:04X}).", opcode);
                    println!("Raw Payload: {:02X?}", payload);
                    // In the future, we parse the server list here
                    return Ok(()); 
                }
                0x0003 | 0x0017 | 0x0018 => println!("✅ Login Accepted (OpCode 0x{:04X}).", opcode),
                _ => println!("Received OpCode 0x{:04X} ({} bytes)", opcode, payload.len()),
            }
        }
    }

    /// Helper: Generic Sender using our new Trait
    async fn send<P: RoF2Packet>(&mut self, packet: P) -> Result<(), Box<dyn std::error::Error>> 
    where 
        for<'a> <P as BinWrite>::Args<'a>: Default 
    {
        let bytes = packet.serialize()?; // Auto-handles size/opcode/endianness
        self.stream.write_all(&bytes).await?;
        Ok(())
    }

    /// Helper: Read full packet with correct framing
    /// Framing: [Size: u16_be] [Body: Size bytes]
    /// Body:    [OpCode: u16_be] [Payload]
    async fn read_packet(&mut self) -> Result<(u16, Vec<u8>), std::io::Error> {
        // 1. Read the Packet Size (First 2 Bytes)
        let mut size_buf = [0u8; 2];
        match self.stream.read_exact(&mut size_buf).await {
            Ok(_) => {},
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Server closed connection"));
            }
            Err(e) => return Err(e),
        }
        let packet_size = u16::from_be_bytes(size_buf);

        // 2. Read the Packet Body (Size Bytes)
        let mut body_buf = vec![0u8; packet_size as usize];
        self.stream.read_exact(&mut body_buf).await?;

        // 3. Extract OpCode (First 2 Bytes of Body)
        if body_buf.len() < 2 {
            return Ok((0x0000, body_buf)); 
        }

        let opcode = u16::from_le_bytes([body_buf[0], body_buf[1]]);
        let payload = body_buf[2..].to_vec();
        
        // DEBUG: Print Packet
        println!("<< Rx OpCode: 0x{:04X} | Size: {} | Payload: {:02X?}", opcode, packet_size, payload);

        Ok((opcode, payload))
    }

    /// Helper: Convert string to [u8; 64]
    fn string_to_bytes(&self, s: &str) -> [u8; 64] {
        let mut buf = [0u8; 64];
        let bytes = s.as_bytes();
        let len = bytes.len().min(64);
        buf[..len].copy_from_slice(&bytes[..len]);
        buf
    }
}
