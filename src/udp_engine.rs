use crate::packets::{ReliableOp, ReliableStreamConnect, ReliableStreamConnectReply};
use binrw::{BinRead, BinWrite};
use std::io::Cursor;
use std::net::SocketAddr;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use async_recursion::async_recursion;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

pub struct ReliableConnection {
    pub socket: Arc<UdpSocket>,
    pub remote_addr: SocketAddr,
    pub status: ConnectionStatus,
    
    // Sequencing
    pub sequence_in: u16,  // Next expected incoming sequence
    pub sequence_out: u16, // Next outgoing sequence
    pub out_of_order_buffer: BTreeMap<u16, Vec<u8>>,
    
    // Connection Params
    pub connect_code: u32, // My Session ID (Randomly generated)
    pub encode_key: u32,   // Given by server (0 initially)
    pub crc_bytes: u8,     // Bytes for CRC (usually 2 or 4)

    // Fragment reassembly
    pub fragment_buffer: Vec<u8>,
    pub fragment_total_size: usize,

    // Encoding
    pub encode_pass1: u8,
    pub encode_pass2: u8,
}

impl ReliableConnection {
    pub fn new(socket: Arc<UdpSocket>, remote_addr: SocketAddr) -> Self {
        Self {
            socket,
            remote_addr,
            status: ConnectionStatus::Disconnected,
            sequence_in: 0,
            sequence_out: 0,
            connect_code: rand::random::<u32>(), 
            encode_key: 0,
            crc_bytes: 0,
            fragment_buffer: Vec::new(),
            fragment_total_size: 0,
            encode_pass1: 0,
            encode_pass2: 0,
            out_of_order_buffer: BTreeMap::new(),
        }
    }

    /// Takes ownership of the connection logic to run the handshake
    pub async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🤝 Starting UDP Handshake with {}", self.remote_addr);
        
        // 1. Send SessionRequest (OpCode 0x01)
        let request = ReliableStreamConnect {
            // Header is implicitly handled by write logic buffer construction if we were doing it manually
            // But we'll construct the buffer manually for now to include the 2-byte header
            protocol_version: 3,
            connect_code: self.connect_code,
            max_packet_size: 512,
            zero: 0,
            opcode: ReliableOp::SessionRequest as u8,
        };

        // Serialize
        let mut writer = Cursor::new(Vec::new());
        request.write(&mut writer)?;
        let packet_bytes = writer.into_inner();

        self.socket.send_to(&packet_bytes, self.remote_addr).await?;
        self.status = ConnectionStatus::Connecting;
        // println!(">> Sent SessionRequest (SessionID: {})", self.connect_code);

        // 2. Wait for SessionResponse (OpCode 0x02)
        // In a real engine, this would be in a receive loop. For handshake blocking, we do it here.
        let mut buf = [0u8; 1024];
        loop {
            let (len, src) = self.socket.recv_from(&mut buf).await?;
            // println!("<< Received {} bytes from {}", len, src);
            if src != self.remote_addr { 
                continue; 
            }
            if len < 2 { continue; }

            // Peek at OpCode
            let opcode = buf[1];
            if opcode == ReliableOp::SessionResponse as u8 {
                print!("<< ConnectReply Hex: ");
                for b in &buf[..len] { print!("{:02X} ", b); }
                println!();
                let mut reader = Cursor::new(&buf[..len]);
                let response = ReliableStreamConnectReply::read(&mut reader)?;
                
                // Validate Session ID match
                if response.connect_code == self.connect_code {
                    self.encode_key = response.encode_key;
                    self.crc_bytes = response.crc_bytes;
                    self.encode_pass1 = response.encode_pass1;
                    self.encode_pass2 = response.encode_pass2;
                    
                    // CRITICAL: Zone Servers (Titanium+) communicate their INITIAL SEQUENCE NUMBER
                    // in what Login Servers use as CRC/Pass fields.
                    // Login Servers always use CRC=2, PASS1=0, PASS2=0 and start at Seq 0.
                    if response.crc_bytes != 2 {
                        let starting_seq = ((response.crc_bytes as u16) << 8) | (response.encode_pass1 as u16);
                        self.sequence_in = starting_seq;
                        println!("✅ Zone Handshake Complete! Key: 0x{:08X}, StartSeq: {}", self.encode_key, starting_seq);
                    } else {
                        self.sequence_in = 0;
                        println!("✅ Handshake Complete! Key: 0x{:08X}, CRC: {}, P1: {}, P2: {}", 
                            self.encode_key, response.crc_bytes, response.encode_pass1, response.encode_pass2);
                    }

                    self.status = ConnectionStatus::Connected;
                    break Ok(());
                }
            }
        }

    }

    /// Wraps an Application Packet (Login Protocol) in a Reliable Stream Packet
    pub async fn send_app_packet(&mut self, app_opcode: u16, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Construct Application Layer: [OpCode: u16_le] [Payload]
        let mut app_blob = Vec::new();
        app_blob.extend_from_slice(&app_opcode.to_le_bytes());
        app_blob.extend_from_slice(payload);

        // 2. Construct Reliable Header (OpCode 0x09 = Packet)
        let seq = self.sequence_out;
        self.sequence_out += 1;

        let header = crate::packets::ReliableStreamReliableHeader {
            zero: 0,
            opcode: ReliableOp::Packet as u8,
            sequence: seq,
        };

        let mut writer = Cursor::new(Vec::new());
        header.write(&mut writer)?;
        let header_bytes = writer.into_inner();

        // 3. Combine [ReliableHeader] [AppBlob]
        let mut reliable_packet = Vec::new();
        reliable_packet.extend_from_slice(&header_bytes);
        reliable_packet.extend_from_slice(&app_blob);

        // 4. Send via internal_send (handles wrapping/CRC)
        self.send_raw_reliable(&reliable_packet).await?;
        // println!(">> Sent App Packet 0x{:04X} (Seq: {})", app_opcode, seq);

        Ok(())
    }

    /// Sends an ACK for a specific sequence number on a specific stream
    pub async fn send_ack(&mut self, stream_id: u8, seq: u16) -> Result<(), Box<dyn std::error::Error>> {
        let opcode = ReliableOp::Ack as u8 + stream_id; // OP_Ack, OP_Ack2, etc.
        
        let header = crate::packets::ReliableStreamReliableHeader {
            zero: 0,
            opcode,
            sequence: seq,
        };

        let mut writer = Cursor::new(Vec::new());
        header.write(&mut writer)?;
        let packet_bytes = writer.into_inner();

        self.send_raw_reliable(&packet_bytes).await?;
        Ok(())
    }

    /// Centralized sending method that handles Encoding (Wrapping) and CRC
    pub async fn send_raw_reliable(&mut self, reliable_packet: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if reliable_packet.len() < 2 { return Ok(()); }
        
        let mut final_packet = reliable_packet.to_vec();
        let opcode = final_packet[1];

        // 1. Encoding (Wrapping/Compression) if data packet (0x03..0x1C)
        // NOTE: 0x1D (OP_OutOfSession) is a control packet — no CRC/encoding
        if (self.status == ConnectionStatus::Connected || self.status == ConnectionStatus::Connecting) 
            && opcode >= 0x03 && opcode <= 0x1C 
        {
             if self.encode_pass1 == 1 || self.encode_pass2 == 1 {
                 let mut wrapped = vec![final_packet[0], final_packet[1]];
                 wrapped.push(0xA5); // Tag for "Uncompressed but wrapped"
                 wrapped.extend_from_slice(&final_packet[2..]);
                 final_packet = wrapped;
             }
        }
        
        // 2. Append CRC
        if self.crc_bytes == 2 {
            let crc = crate::crc::crc32_with_key(&final_packet, self.encode_key);
            let crc_low = (crc & 0xffff) as u16;
            final_packet.extend_from_slice(&crc_low.to_be_bytes());
        } else if self.crc_bytes == 4 {
             let crc = crate::crc::crc32_with_key(&final_packet, self.encode_key);
             final_packet.extend_from_slice(&crc.to_be_bytes());
        }
        
        // Hex log outgoing
        if opcode >= 0x03 {
            // print!(">> Sent RelOp: 0x{:02X}, Len: {}, Hex: ", opcode, final_packet.len());
            // for b in &final_packet { print!("{:02X} ", b); }
            // println!();
        }

        self.socket.send_to(&final_packet, self.remote_addr).await?;
        Ok(())
    }

    /// Entry point for processing a raw received UDP packet
    #[async_recursion::async_recursion]
    pub async fn process_incoming_packet(&mut self, buf: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut app_payloads = Vec::new();
        if buf.len() < 2 { return Ok(app_payloads); }
        
        let mut data = buf.to_vec();

        // =========================================================
        // NON-RELIABLE APPLICATION PACKETS (first byte != 0)
        // These bypass the session/reliability layer entirely.
        // Format: [app_data_byte0] [compress_flag] [app_data_rest...] [CRC]
        // After CRC strip + decompress: raw application payload
        // =========================================================
        if data[0] != 0x00 {
            if self.status != ConnectionStatus::Connected && self.status != ConnectionStatus::Connecting {
                return Ok(app_payloads);
            }

            // LOG
            // print!("<< Rx NonRel Raw Hex ({} bytes): ", data.len());
            // for b in data.iter().take(32) { print!("{:02X} ", b); }
            // if data.len() > 32 { print!("..."); }
            // println!();

            // CRC Validation & Strip
            if self.crc_bytes > 0 && data.len() > self.crc_bytes as usize {
                let end = data.len() - self.crc_bytes as usize;
                let rx_crc: u32 = if self.crc_bytes == 2 {
                    u16::from_be_bytes([data[data.len()-2], data[data.len()-1]]) as u32
                } else {
                    u32::from_be_bytes([data[data.len()-4], data[data.len()-3], data[data.len()-2], data[data.len()-1]])
                };
                let calc_crc = crate::crc::crc32_with_key(&data[..end], self.encode_key);
                let calc_val = if self.crc_bytes == 2 { (calc_crc & 0xFFFF) as u32 } else { calc_crc };

                if rx_crc != calc_val {
                    println!("❌ NonRel CRC MISMATCH: Rx=0x{:X}, Calc=0x{:X}, Len={}", rx_crc, calc_val, data.len());
                    return Ok(app_payloads);
                }
                data.truncate(end);
            }

            // Decompress: flag byte at offset 1
            if (self.encode_pass1 == 1 || self.encode_pass2 == 1) && data.len() > 1 {
                let tag = data[1];
                if tag == 0x5a {
                    // Compressed: decompress data[2..]
                    match decompress_zlib(&data[2..]) {
                        Ok(decompressed) => {
                            let mut new_data = vec![data[0]];
                            new_data.extend_from_slice(&decompressed);
                            data = new_data;
                        }
                        Err(e) => {
                            println!("⚠️ NonRel decompression failed: {}. Len: {}", e, data.len());
                            return Ok(app_payloads);
                        }
                    }
                } else if tag == 0xa5 {
                    // Uncompressed: strip the A5 flag byte
                    let mut new_data = vec![data[0]];
                    new_data.extend_from_slice(&data[2..]);
                    data = new_data;
                }
                // else: unknown tag, pass through as-is
            }

            // The decoded data is a raw application payload (2-byte LE opcode + payload)
            if data.len() >= 2 {
                let app_op = u16::from_le_bytes([data[0], data[1]]);
                println!("   NonRel App Packet: Op=0x{:04X}, Size={}", app_op, data.len());
                app_payloads.push(data);
            }
            return Ok(app_payloads);
        }

        // =========================================================
        // SESSION-LEVEL PACKETS (first byte == 0)
        // =========================================================
        let opcode = data[1];

        // LOG EVERY INCOMING PACKET RAW HEX
        if opcode != 0x02 { // Hide session response spam
            // print!("<< Rx Raw Hex: ");
            // for b in &data { print!("{:02X} ", b); }
            // println!();
        }

        // 1. Differentiate between control and data packets for decoding
        // SessionRequest/Response/Disconnect/KeepAlive/OutOfSession are NOT encoded/compressed.
        // Data packets (0x03..=0x1C) might be. 0x1D (OP_OutOfSession) is a control packet.
        if (self.status == ConnectionStatus::Connected || self.status == ConnectionStatus::Connecting) 
            && opcode >= 0x03 && opcode <= 0x1C 
        {
            // First, remove CRC before decoding
            let end = if data.len() > self.crc_bytes as usize { data.len() - self.crc_bytes as usize } else { data.len() };
            
            // CRC Validation Log
            if self.crc_bytes > 0 && data.len() >= self.crc_bytes as usize {
                let rx_crc: u32 = if self.crc_bytes == 2 {
                    u16::from_be_bytes([data[data.len()-2], data[data.len()-1]]) as u32
                } else {
                    u32::from_be_bytes([data[data.len()-4], data[data.len()-3], data[data.len()-2], data[data.len()-1]])
                };
                let calc_crc = crate::crc::crc32_with_key(&data[..end], self.encode_key);
                let calc_val = if self.crc_bytes == 2 { (calc_crc & 0xFFFF) as u32 } else { calc_crc };
                
                if rx_crc != calc_val {
                    println!("❌ CRC MISMATCH: Op=0x{:02x}, Rx=0x{:X}, Calc=0x{:X}, Len={}", opcode, rx_crc, calc_val, data.len());
                } else {
                    // println!("✅ CRC OK: Op=0x{:02x}", opcode);
                }
            }

            data.truncate(end);

            if self.encode_pass1 == 1 || self.encode_pass2 == 1 {
                // Decompress everything from byte 2 onwards (session header is [00 opcode])
                if data.len() > 2 {
                    let tag = data[2];
                    if tag == 0x5a {
                        // Compressed
                        let payload = &data[3..];
                        match decompress_zlib(payload) {
                            Ok(decompressed) => {
                                let mut new_data = vec![data[0], data[1]];
                                new_data.extend_from_slice(&decompressed);
                                data = new_data;
                            }
                            Err(e) => {
                                println!("⚠️ Decompression failed: {}. Op: 0x{:02x}, Len: {}", e, opcode, data.len());
                            }
                        }
                    } else if tag == 0xa5 {
                        // Uncompressed but wrapped
                        let mut new_data = vec![data[0], data[1]];
                        new_data.extend_from_slice(&data[3..]);
                        data = new_data;
                    }
                }
            }
        }

        let len = data.len();
        if opcode != 0x02 { // Ignore session response spam
             if (0x09..=0x18).contains(&opcode) && data.len() >= 4 {
                 let seq_val = u16::from_be_bytes([data[2], data[3]]);
                 // println!("   Rx RelOp: 0x{:02x}, Len: {}, Seq: {} ({:02x} {:02x}), buf[0]={:02x}", opcode, len, seq_val, data[2], data[3], data[0]);
             } else {
                 // println!("   Rx RelOp: 0x{:02x}, Len: {}, buf[0]={:02x}", opcode, len, data[0]);
             }
        }
        
        match opcode {
            0x09 | 0x0a | 0x0b | 0x0c => { // OP_Packet...
                if data.len() < 4 { return Ok(app_payloads); }
                let seq = u16::from_be_bytes([data[2], data[3]]);
                // DON'T ack here — ack handled inside handle_sequenced_packet
                // so we only ack the highest contiguous seq
                let stream_id = opcode - 0x09;
                app_payloads.extend(self.handle_sequenced_packet(seq, opcode, &data[4..], stream_id).await?);
            }
            0x0d | 0x0e | 0x0f | 0x10 => { // OP_Fragment
                if data.len() < 4 { return Ok(app_payloads); }
                let seq = u16::from_be_bytes([data[2], data[3]]);
                let stream_id = opcode - 0x0d;
                app_payloads.extend(self.handle_sequenced_packet(seq, opcode, &data[4..], stream_id).await?);
            }
            0x03 => { // OP_Combined
                let mut offset = 2;
                while offset < data.len() {
                    let sub_len = data[offset] as usize;
                    offset += 1;
                    if offset + sub_len > data.len() { break; }
                    let sub_res = self.process_subpacket(&data[offset..offset+sub_len]).await?;
                    app_payloads.extend(sub_res);
                    offset += sub_len;
                }
            }
            0x1d => { // OP_OutOfSession — session-level control, no action needed
            }
            _ => {}
        }
        
        Ok(app_payloads)
    }

    /// Handles a combined subpacket (recursively)
    #[async_recursion::async_recursion]
    async fn process_subpacket(&mut self, buf: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut app_payloads = Vec::new();
        if buf.len() < 2 { return Ok(app_payloads); }
        let opcode = buf[1];
        
        match opcode {
            0x09 | 0x0a | 0x0b | 0x0c => {
                if buf.len() < 4 { return Ok(app_payloads); }
                let seq = u16::from_be_bytes([buf[2], buf[3]]);
                let stream_id = opcode - 0x09;
                app_payloads.extend(self.handle_sequenced_packet(seq, opcode, &buf[4..], stream_id).await?);
            }
            0x0d => {
                 if buf.len() < 4 { return Ok(app_payloads); }
                 let seq = u16::from_be_bytes([buf[2], buf[3]]);
                 app_payloads.extend(self.handle_sequenced_packet(seq, opcode, &buf[4..], 0).await?);
            }
            0x03 => {
                 let mut offset = 2;
                 while offset < buf.len() {
                     let sub_len = buf[offset] as usize;
                     offset += 1;
                     if offset + sub_len > buf.len() { break; }
                     let sub_sub = &buf[offset..offset+sub_len];
                     let res = self.process_subpacket(sub_sub).await?;
                     app_payloads.extend(res);
                     offset += sub_len;
                 }
            }
            _ => {}
        }
        Ok(app_payloads)
    }

    /// Enforces strict sequence ordering. Buffers out-of-order packets.
    /// Only ACKs the highest contiguous sequence to force retransmission of gaps.
    async fn handle_sequenced_packet(&mut self, seq: u16, opcode: u8, payload: &[u8], stream_id: u8) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let diff = seq.wrapping_sub(self.sequence_in);
        // println!("DEBUG: seq={}, expected={}, diff={}", seq, self.sequence_in, diff);
        
        if diff == 0 {
            // Found expected packet
            self.sequence_in = self.sequence_in.wrapping_add(1);
            let mut results = self.process_payload(opcode, payload)?;
            
            // Drain buffer for subsequent packets
            while self.out_of_order_buffer.contains_key(&self.sequence_in) {
                if let Some(buffered) = self.out_of_order_buffer.remove(&self.sequence_in) {
                    let op = buffered[0];
                    let load = &buffered[1..];
                    self.sequence_in = self.sequence_in.wrapping_add(1);
                    results.extend(self.process_payload(op, load)?);
                }
            }
            
            // ACK the highest contiguous sequence we've processed
            let ack_seq = self.sequence_in.wrapping_sub(1);
            self.send_ack(stream_id, ack_seq).await?;
            
            Ok(results)
        } else if diff < 32768 {
             // Future packet (buffer it, do NOT ack — server will resend the gap)
             println!("⚠️  Buffering Out-of-Order: Seq={} (Expected={})", seq, self.sequence_in);
             let mut buf = Vec::with_capacity(1 + payload.len());
             buf.push(opcode);
             buf.extend_from_slice(payload);
             self.out_of_order_buffer.insert(seq, buf);
             Ok(Vec::new())
        } else {
             // Past packet (duplicate)
             // println!("♻️  Ignoring Duplicate: Seq={}", seq);
             Ok(Vec::new())
        }
    }

    /// Processes the payload of a sequenced packet (AppPacket or Fragment)
    fn process_payload(&mut self, opcode: u8, payload: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut app_payloads = Vec::new();

        match opcode {
            0x09..=0x0c => { // OP_Packet
                app_payloads.push(payload.to_vec());
            }
            0x0d => { // OP_Fragment
                // payload[0..end] is the fragment data (header already stripped)
                // BUT wait, fragment logic needs to know if it's the start
                // The start fragment (if buffer empty) has TotalSize at payload[0..4]
                
                if self.fragment_buffer.is_empty() {
                    if payload.len() >= 4 {
                         let possible_size = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
                         if possible_size > 0 {
                             self.fragment_total_size = possible_size;
                             self.fragment_buffer.extend_from_slice(&payload[4..]); // Skip size
                             println!("🚀 STARTING NEW REASSEMBLY: Total Size={}", self.fragment_total_size);
                         }
                    }
                } else {
                    self.fragment_buffer.extend_from_slice(payload);
                }
                
                if !self.fragment_buffer.is_empty() && self.fragment_buffer.len() >= self.fragment_total_size {
                    println!("📦 PACKET REASSEMBLED! Size: {}", self.fragment_buffer.len());
                    app_payloads.push(self.fragment_buffer.clone());
                    self.fragment_buffer.clear();
                    self.fragment_total_size = 0;
                }
            }
            _ => {}
        }
        Ok(app_payloads)
    }
}

fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    use flate2::read::ZlibDecoder;
    
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
