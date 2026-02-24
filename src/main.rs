mod crypto;
mod udp_engine;
mod packets;
mod crc;
mod zone_state;
mod web_ui;

use udp_engine::ReliableConnection;
use packets::{LoginAppOp, WorldAppOp, ZoneAppOp, LoginBaseMessage, ZoneEntry, ClientZoneEntry, TitaniumCharInfo, SpawnStruct, PlayerProfile, ClientUpdate};
use zone_state::{ZoneState, Spawn};
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use binrw::{BinRead, BinWrite};
use std::io::Cursor;
use std::io::Write;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, mpsc};
use packets::ClientUpdateClient;
use web_ui::GameCommand;

async fn prompt(msg: &str) -> io::Result<String> {
    print!("{}", msg);
use std::io::Write;
    std::io::stdout().flush()?;
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

async fn prompt_secret(msg: &str) -> io::Result<String> {
    print!("{}", msg);
    use std::io::Write;
    std::io::stdout().flush()?;
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let login_server_ip = "192.168.1.21:5998"; 
    let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
    socket.set_recv_buffer_size(2 * 1024 * 1024)?; // 2MB
    socket.bind(&"0.0.0.0:0".parse::<std::net::SocketAddr>()?.into())?;
    socket.set_nonblocking(true)?;
    let socket = tokio::net::UdpSocket::from_std(socket.into())?;
    let socket = Arc::new(socket);

    // Initialize Zone State & Web UI
    let zone_state = Arc::new(Mutex::new(ZoneState::new()));
    let (tx, _rx) = broadcast::channel(100);
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<GameCommand>();
    
    let web_state = zone_state.clone();
    let web_tx = tx.clone();
    tokio::spawn(async move {
        web_ui::start_web_server(web_state, web_tx, cmd_tx).await;
    });
    
    println!("🚀 Mobile Client Started.");
    println!("🌐 Web UI available at http://localhost:3030");
    println!("🔌 Connecting to Login Server at {}", login_server_ip);

    let remote_addr = login_server_ip.parse()?;
    let mut connection = ReliableConnection::new(socket.clone(), remote_addr);

    // 2. Perform Handshake
    connection.handshake().await?;

    // 3. Send OP_SessionReady
    connection.send_app_packet(LoginAppOp::SessionReady as u16, &[0u8; 4]).await?;
    println!(">> Sent OP_SessionReady");

    // 4. Send OP_Login
    // 2. Interactive Login
    // let username = prompt("Username: ").await?;
    // let password = prompt_secret("Password: ").await?;
    let username = "tmc".to_string();
    let password = "tmc".to_string();

    let mut creds_blob = Vec::new();
    creds_blob.extend_from_slice(username.as_bytes());
    creds_blob.push(0);
    creds_blob.extend_from_slice(password.as_bytes());
    creds_blob.push(0);

    while creds_blob.len() % 8 != 0 {
        creds_blob.push(0);
    }
    let encrypted_creds = crypto::encrypt_null_des(&creds_blob);

    let base_msg = LoginBaseMessage {
        sequence: 3, 
        compressed: 0,
        encrypt_type: 2, 
        unk3: 0,
    };
    
    let mut writer = Cursor::new(Vec::new());
    base_msg.write(&mut writer)?;
    let mut login_payload = writer.into_inner();
    login_payload.extend_from_slice(&encrypted_creds);

    connection.send_app_packet(LoginAppOp::Login as u16, &login_payload).await?;
    println!(">> Sent OP_Login (User: {})", username);

    let mut session_key = String::new();
    let mut lsid = 0;
    let mut selected_server_ip = String::new();
    let mut selected_server_id = 0;
    let mut server_selected = false;
    let mut char_name_for_zone = String::new();
    let mut zone_server_ip = String::new();
    let mut zone_server_port: u16 = 0;

    // 5. Login Receiver Loop
    'login_loop: loop {
        let mut buf = [0u8; 4096];
        let (len, src) = socket.recv_from(&mut buf).await?;
        if src != remote_addr { continue; }

        let payloads = connection.process_incoming_packet(&buf[..len]).await?;
        for payload in payloads {
            if payload.len() < 2 { continue; }
            let app_opcode = u16::from_le_bytes([payload[0], payload[1]]);

            if app_opcode == LoginAppOp::LoginAccepted as u16 {
                println!("🎉 LOGIN ACCEPTED!");
                let header_size = 2 + 10;
                if payload.len() > header_size {
                    let decrypted = crypto::decrypt_null_des(&payload[header_size..]);
                    let mut reader = Cursor::new(&decrypted);
                    if let Ok(reply) = packets::PlayerLoginReply::read(&mut reader) {
                        session_key = String::from_utf8_lossy(&reply.key).trim_matches(char::from(0)).to_string();
                        lsid = reply.lsid;
                        println!("🔑 Session Key: {}, LSID: {}", session_key, lsid);
                        
                        println!("📑 Requesting Server List...");
                        let sl_req = packets::ServerListRequest { unknown: 0 };
                        let mut writer = Cursor::new(Vec::new());
                        sl_req.write(&mut writer)?;
                        connection.send_app_packet(LoginAppOp::ServerListRequest as u16, &writer.into_inner()).await?;
                    }
                }
            } else if app_opcode == LoginAppOp::ServerListResponse as u16 {
                if server_selected { continue; }
                println!("📜 Received Server List!");
                let mut reader = Cursor::new(&payload[2..]);
                if let Ok(sl) = packets::ServerListReply::read(&mut reader) {
                    println!("Available Servers:");
                    for srv in sl.servers.iter() {
                        println!("  [{}] {} (Status: {})", srv.server_id, srv.server_name, srv.server_status);
                    }
                    
                    // let input = prompt("Enter Server ID to join: ").await?;
                    // let target_id = input.parse::<u32>().unwrap_or(0);
                    let target_id = 1;

                    if let Some(srv) = sl.servers.iter().find(|s| s.server_id as u32 == target_id) {
                        selected_server_id = srv.server_id as u32;
                        selected_server_ip = srv.ip.clone();
                        server_selected = true;
                        println!("🚀 Joining {} ({})", srv.server_name, selected_server_ip);
                        
                        let play_req = packets::PlayEverquestRequest {
                            base_header: LoginBaseMessage { sequence: 5, compressed: 0, encrypt_type: 0, unk3: 0 },
                            server_number: selected_server_id,
                        };
                        let mut writer = Cursor::new(Vec::new());
                        play_req.write(&mut writer)?;
                        connection.send_app_packet(LoginAppOp::PlayEverquestRequest as u16, &writer.into_inner()).await?;
                    } else {
                        println!("❌ Invalid Server ID.");
                    }
                }
            }
            else if app_opcode == LoginAppOp::PlayEverquestResponse as u16 {
                println!("🛸 Received Play Response! Proceeding to World Handoff...");
                break 'login_loop; 
            }
        }
    }

    // 6. Connect to World Server
    let world_addr_str = format!("{}:9000", selected_server_ip);
    let world_addr: SocketAddr = world_addr_str.parse()?;
    println!("🔌 Connecting to World Server at {}", world_addr);
    
    let mut world_conn = ReliableConnection::new(socket.clone(), world_addr);
    world_conn.handshake().await?;

    // 7. Send OP_SendLoginInfo
    println!("🛂 Sending login info to World Server...");
    let mut info_buf = vec![0u8; 64];
    let payload_str = format!("{}\0{}\0", lsid, session_key);
    let ph_bytes = payload_str.as_bytes();
    if ph_bytes.len() <= 64 {
        info_buf[..ph_bytes.len()].copy_from_slice(ph_bytes);
    }

    let world_login = packets::LoginInfo {
        login_info: info_buf,
        unknown064: vec![0u8; 124],
        zoning: 0,
        unknown189: vec![0u8; 275],
    };
    
    let mut writer = Cursor::new(Vec::new());
    world_login.write(&mut writer)?;
    world_conn.send_app_packet(WorldAppOp::SendLoginInfo as u16, &writer.into_inner()).await?;

    // 8. World Receiver Loop (Wait for CharInfo)
    println!("⏳ Waiting for World Approval and Character List...");
    let mut got_zone_entry = false;
    let mut zone_id_for_zone: u32 = 0;
    let mut zone_name_for_zone = String::new();
    let mut pending_char_info: Option<(usize, String, u8, u32, u32)> = None;
    'world_loop: loop {
        let mut buf = [0u8; 4096];
        let (len, src) = socket.recv_from(&mut buf).await?;
        
        if src != world_addr && src != remote_addr {
            continue;
        }

        if src == remote_addr {
             println!("<< Received {} bytes from Login Server (Trailing)", len);
             continue;
        }

        // println!("<< Received {} bytes from World Server", len);
        // DEBUG: Dump raw bytes - DISABLED for performance
        // print!("   UDP RAW: ");
        // for b in buf.iter().take(len) {
        //     print!("{:02X} ", b);
        // }
        // println!();

        let payloads = world_conn.process_incoming_packet(&buf[..len]).await?;
        for payload in payloads {
            if payload.len() < 2 { continue; }
            let app_opcode = u16::from_le_bytes([payload[0], payload[1]]);

            if app_opcode == WorldAppOp::ApproveWorld as u16 {
                println!("🌎 WORLD APPROVED! We are in!");
            } else if app_opcode == WorldAppOp::LogServer as u16 {
                println!("ℹ️  Received LogServer packet");
            } else if app_opcode == WorldAppOp::PostEnterWorld as u16 {
                println!("🚀 POST ENTER WORLD (Ready for character selection)!");
                if let Some(char_info) = pending_char_info.take() {
                    let index = char_info.0;
                    let name = char_info.1;
                    
                    println!("🚀 ENTERING WORLD with character: {}", name);
                    
                    let mut name_buf = vec![0u8; 64];
                    let name_bytes = name.as_bytes();
                    for (j, b) in name_bytes.iter().enumerate().take(64) {
                        name_buf[j] = *b;
                    }
                    
                    let mut payload = Vec::with_capacity(72);
                    payload.extend_from_slice(&name_buf); // 64 bytes
                    payload.extend_from_slice(&[0, 0, 0, 0]); // Tutorial (u32 LE)
                    payload.extend_from_slice(&[0, 0, 0, 0]); // ReturnHome (u32 LE)
                    
                    println!(">> Sending EnterWorld for {} (index {})", name, index);
                    world_conn.send_app_packet(WorldAppOp::EnterWorld as u16, &payload).await?;
                }
            } else if app_opcode == WorldAppOp::SendCharInfo as u16 || app_opcode == WorldAppOp::SendCharInfoNew as u16 {
                println!("🎭 RECEIVED CHARACTER INFO! (Op: 0x{:04x}, Size: {})", app_opcode, payload.len());
                let app_data = &payload[2..]; 
                
                // Titanium / RoF2 format: Fixed-size 1704 bytes
                // struct TitaniumCharInfo
                
                let mut available_chars: Vec<(usize, String, u8, u32, u32)> = Vec::new(); // (index, name, level, race, zone_id)

                // Try to parse as TitaniumCharInfo
                let mut reader = Cursor::new(app_data);
                if let Ok(char_info) = TitaniumCharInfo::read(&mut reader) {
                    println!("   Successfully parsed TitaniumCharInfo!");
                    
                    for i in 0..10 {
                        // Check name
                        let name_bytes = &char_info.names[i];
                        // Convert to string and trim nulls
                        let name = String::from_utf8_lossy(name_bytes).trim_matches('\0').to_string();
                        
                        if !name.is_empty() && name != "<none>" {
                            let race = char_info.race[i];
                            let class = char_info.class[i];
                            let level = 1; 
                            let zone_id = char_info.zone[i];
                            
                            println!("   [{}] Name: '{}', Class: {}, Race: {}, Zone: {}", i, name, class, race, zone_id);
                            available_chars.push((i, name, level, race, zone_id));
                        }
                    }
                } else {
                    println!("   ❌ Failed to parse TitaniumCharInfo!");
                }

                    if !available_chars.is_empty() {
                        let char_idx = 0; 
                        
                        // Find name for index
                        if let Some((idx, name, level, race, zid)) = available_chars.iter().find(|(idx, _, _, _, _)| *idx == char_idx) {
                             println!("   Character identified: {}. Waiting for PostEnterWorld...", name);
                             char_name_for_zone = name.clone();
                             zone_id_for_zone = *zid;
                             pending_char_info = Some((*idx, name.clone(), *level, *race, *zid));
                        }
                    } else {
                        println!("⚠️ No characters found. Using override 'Juggs'...");
                        char_name_for_zone = "Juggs".to_string();
                        pending_char_info = Some((0, "Juggs".to_string(), 1, 2, 29));
                    }


            } else if app_opcode == WorldAppOp::ZoneServerInfo as u16 {
                println!("🌍 RECEIVED ZONE SERVER INFO! (Op: 0x{:04x})", app_opcode);
                let app_data = &payload[2..];
                 // Hex dump
                 print!("   Hex: ");
                 for b in app_data { print!("{:02X} ", b); }
                 println!("");

                 // Parse IP: starts at offset 0 of app_data
                 let ip_str = String::from_utf8_lossy(&app_data[..128.min(app_data.len())]).trim_matches(char::from(0)).to_string();
                 
                 // Parse Port: offset 128 (u16)
                 let mut port = 7000;
                 if app_data.len() >= 130 {
                     port = u16::from_le_bytes([app_data[128], app_data[129]]);
                 }

                 println!("   Zone Server IP: {}", ip_str);
                 println!("   Zone Server Port: {}", port);
                 
                 zone_server_ip = ip_str;
                 zone_server_port = port;
                 
                 got_zone_entry = true;
                 break 'world_loop; 

            } else if app_opcode == WorldAppOp::ZoneEntry as u16 { 
                 println!("🌍 RECEIVED ZONE ENTRY! (Op: 0x{:04x})", app_opcode);
                 let mut data_reader = Cursor::new(&payload[2..]);
                 match ZoneEntry::read(&mut data_reader) {
                    Ok(zone_entry) => {
                         let zone_name = String::from_utf8_lossy(&zone_entry.zone_short_name).trim_matches('\0').to_string();
                         let ip_str = String::from_utf8_lossy(&zone_entry.ip).trim_matches('\0').to_string();
                         char_name_for_zone = String::from_utf8_lossy(&zone_entry.char_name).trim_matches('\0').to_string();
                         zone_server_ip = ip_str.clone();
                         zone_server_port = zone_entry.port as u16;
                         zone_name_for_zone = zone_name.clone();
                         zone_id_for_zone = zone_entry.zone_id;
                         
                         println!("🌟 ZONING APPROVED!");
                         println!("   Zone: {} (ID: {})", zone_name, zone_entry.zone_id);
                         println!("   Zone Server: {}:{}", ip_str, zone_server_port);
                         println!("   Character: {}", char_name_for_zone);
                         got_zone_entry = true;
                         break 'world_loop;
                    }
                    Err(e) => println!("   ❌ Failed to parse ZoneEntry: {:?}", e),
                 }
            } else {
                println!("📩 World App Packet: Op=0x{:04X}, Size={}", app_opcode, payload.len());
                
                if payload.len() == 130 {
                    println!("🎯 FOUND 130-BYTE PACKET (ZoneServerInfo size) in 0x{:04X}!", app_opcode);
                }

                print!("   Hex: ");
                for b in payload.iter().take(32) {
                    print!("{:02X} ", b);
                }
                println!();
            }
        }
    }

    // ================================================================
    // 9. ZONE SERVER CONNECTION
    // ================================================================
    let zone_addr_str = format!("{}:{}", zone_server_ip, zone_server_port);
    let zone_addr: SocketAddr = zone_addr_str.parse()?;
    println!("\n🔌 Connecting to Zone Server at {}", zone_addr);

    // IMPORTANT: Create a NEW socket for the zone connection.
    // Reusing the world socket causes OP_OutOfSession (0x1D) because
    // the zone server's session tracking conflicts with world server traffic.
    let zone_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let mut zone_conn = ReliableConnection::new(zone_socket.clone(), zone_addr);
    zone_conn.handshake().await?;
    println!("✅ Zone Server UDP handshake complete! Current SeqIn: {}", zone_conn.sequence_in);

    // Send OP_ZoneEntry (ClientZoneEntry_Struct Titanium)
    // Structure: [u32 unknown00] [char Name[64]]
    // Total: 68 bytes
    let mut payload = Vec::with_capacity(68);

    // unknown00 (4 bytes) - Titanium: often 0
    payload.extend_from_slice(&0u32.to_le_bytes()); 

    // char_name[64]
    let mut name_bytes = [0u8; 64];
    println!("   Zoning with Character: {}", char_name_for_zone);
    for (i, b) in char_name_for_zone.as_bytes().iter().take(63).enumerate() {
        name_bytes[i] = *b;
    }
    payload.extend_from_slice(&name_bytes);

    println!(">> Sending Titanium OP_ZoneEntry (Size: {})", payload.len());
    zone_conn.send_app_packet(ZoneAppOp::ZoneEntry as u16, &payload).await?;

    // 10. Zone Handshake State Machine
    println!("⏳ Running zone handshake...");
    
    // Set zone info in ZoneState so web UI shows it immediately
    {
        let mut state = zone_state.lock().unwrap();
        state.set_my_name(char_name_for_zone.clone());
        state.zone_id = zone_id_for_zone;
        state.zone_name = zone_name_for_zone.clone();
    }

    let mut got_player_profile = false;
    let mut got_new_zone = false;
    let mut sent_req_new_zone = false;
    let mut sent_req_client_spawn = false;
    let mut got_world_objects_sent = false;
    let mut sent_client_ready = false;

    let mut update_seq = 0u16;

    'zone_loop: loop {
        let mut buf = [0u8; 8192]; // Larger buffer for PlayerProfile fragments
        
        let mut socket_result = None;
        let mut ws_cmd = None;

        tokio::select! {
            res = zone_socket.recv_from(&mut buf) => {
                socket_result = Some(res);
            }
            Some(cmd) = cmd_rx.recv() => {
                ws_cmd = Some(cmd);
            }
        }

        if let Some(cmd) = ws_cmd {
            let mut packet_to_send = None;
            let mut app_opcode = 0u16;

            match cmd {
                GameCommand::Move { d_x, d_y, d_z, d_h } => {
                    let mut state = zone_state.lock().unwrap();
                    let my_id = state.my_spawn_id.unwrap_or(0) as u16;
                    
                    if let Some(player) = &mut state.player {
                        // Update State
                        player.x += d_x;
                        player.y += d_y;
                        player.z += d_z;
                        player.heading += d_h;

                        // Update sequence
                        update_seq = update_seq.wrapping_add(1);

                        // Create Packet
                        let update = ClientUpdateClient {
                            spawn_id: my_id,
                            sequence: update_seq,
                            y_pos: player.y,
                            x_pos: player.x,
                            z_pos: player.z,
                            delta_x: d_x,
                            delta_y: d_y,
                            delta_z: d_z,
                            delta_heading: 0, 
                            heading: player.heading as u16, 
                            animation: 0,
                        };
                        packet_to_send = Some(update.to_bytes());
                        app_opcode = 0x14CB; // OP_ClientUpdate
                    }
                },
                GameCommand::Target { id } => {
                    println!("🎯 Target Command: ID {}", id);
                    let target_pkt = packets::ClientTarget { target_id: id };
                    packet_to_send = Some(target_pkt.to_bytes());
                    app_opcode = 0x184D; // OP_TargetCommand
                },
                GameCommand::Attack { targetId } => {
                    println!("⚔️ Attack Command - ID: {}", targetId);
                    // Try sending the Target ID as payload
                    packet_to_send = Some(targetId.to_le_bytes().to_vec()); 
                    app_opcode = 0x5e55; // OP_AutoAttack
                },
                GameCommand::Teleport { x, y, z } => {
                    println!("✨ Teleport Command: ({}, {}, {})", x, y, z);
                    let mut state = zone_state.lock().unwrap();
                    let my_id = state.my_spawn_id.unwrap_or(0) as u16;
                    
                    if let Some(player) = &mut state.player {
                        // Force Update State
                        player.x = x;
                        player.y = y;
                        player.z = z;

                        // Update sequence
                        update_seq = update_seq.wrapping_add(1);

                        // Create Packet
                        let update = ClientUpdateClient {
                            spawn_id: my_id,
                            sequence: update_seq,
                            y_pos: y,
                            x_pos: x,
                            z_pos: z,
                            delta_x: 0.0,
                            delta_y: 0.0,
                            delta_z: 0.0,
                            delta_heading: 0, 
                            heading: player.heading as u16, 
                            animation: 0,
                        };
                        packet_to_send = Some(update.to_bytes());
                        app_opcode = 0x14CB; // OP_ClientUpdate
                    }
                }
            }

            if let Some(data) = packet_to_send {
                if let Err(e) = zone_conn.send_app_packet(app_opcode, &data).await {
                     println!("Failed to send packet: {:?}", e);
                }
            }
        }
        
        let payloads = if let Some(res) = socket_result {
             let (len, src) = res?;
             // Dedicated zone socket — no need to filter by source address
             let _ = src;
             zone_conn.process_incoming_packet(&buf[..len]).await?
        } else {
             Vec::new()
        };
        for payload in payloads {
            if payload.len() < 2 { continue; }
            let app_opcode = u16::from_le_bytes([payload[0], payload[1]]);

            if app_opcode == ZoneAppOp::PlayerProfile as u16 {
                println!("📋 Received PlayerProfile ({} bytes)", payload.len());
                got_player_profile = true;
                
                let mut reader = Cursor::new(&payload[2..]);
                if let Ok(profile) = PlayerProfile::read(&mut reader) {
                     println!("   Name: {}, Race: {}, Class: {}", profile.name, profile.race, profile.class);
                     let mut state = zone_state.lock().unwrap();
                     
                     // Helper: clamp u32 to u8 for Spawn struct (which uses legacy fields?)
                     let hp_u8 = (profile.cur_hp.min(255)) as u8;
                     
                     // Initialize player spawn using profile data
                     // Note: Profile doesn't have spawn_id usually, but we can set initial coords
                     if state.player.is_none() {
                         state.player = Some(Spawn {
                             id: 0, 
                             name: profile.name.clone(),
                             level: profile.level,
                             race: profile.race as u16,
                             class: profile.class as u8,
                             hp: hp_u8,
                             max_hp: 100, // Placeholder max
                             x: profile.x,
                             y: profile.y,
                             z: profile.z,
                             heading: profile.heading,
                             is_npc: false,
                         });
                     }
                     state.profile = Some(profile.clone());
                      let _ = std::fs::write("profile_full.bin", &payload[2..]);
                      println!("💾 Dumped profile_full.bin ({} bytes)", payload.len() - 2);
                     println!("📍 Player Coords: X={:.1}, Y={:.1}, Z={:.1}, H={:.1}", profile.x, profile.y, profile.z, profile.heading);
                     let start = 13116.min(payload.len().saturating_sub(2));
                     let end = (payload.len().saturating_sub(2)).min(start + 32);
                     print!("   Player XYZ Hex: ");
                     for b in &payload[2+start..2+end] { print!("{:02X} ", b); }
                     println!();
                     
                     // Dump inventory blob for analysis
                     if let Err(e) = std::fs::write("inventory.bin", &profile.inventory_blob) {
                          println!("⚠️ Failed to write inventory.bin: {}", e);
                     } else {
                          println!("💾 Dumped inventory.bin ({} bytes)", profile.inventory_blob.len());
                     }
                }

            } else if app_opcode == ZoneAppOp::Damage as u16 {
                let mut reader = Cursor::new(&payload[2..]);
                if let Ok(dmg) = packets::CombatDamage::read(&mut reader) {
                    println!("💥 Damage: Src {} -> Tgt {} : {}", dmg.source, dmg.target, dmg.damage);
                    // Broadcast damage event to Web UI
                    let msg = format!(r#"{{"type":"damage","source":{},"target":{},"amount":{}}}"#, 
                        dmg.source, dmg.target, dmg.damage);
                    let _ = tx.send(msg);
                }

            } else if app_opcode == ZoneAppOp::NewZone as u16 {
                // OP_NewZone (0x0920) - Confirmation of actual binary zone metadata
                println!("🗺️  Received Binary NewZone (Op=0x0920, {} bytes)", payload.len());
                got_new_zone = true;
                
                let app_data = &payload[2..];
                if app_data.len() >= 96 {
                    let zone_short = String::from_utf8_lossy(&app_data[64..96]).trim_matches('\0').to_string();
                    if !zone_short.is_empty() && zone_short.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        println!("   ✅ Success! Zone Name: {}", zone_short);
                        let mut state = zone_state.lock().unwrap();
                        state.zone_name = zone_short;
                        state.zone_id = zone_id_for_zone;
                    }
                }

                // Respond with OP_ReqNewZone if we haven't yet
                if !sent_req_new_zone {
                    zone_conn.send_app_packet(ZoneAppOp::ReqNewZone as u16, &[]).await?;
                    println!(">> Sent OP_ReqNewZone");
                    sent_req_new_zone = true;
                }

            } else if app_opcode == ZoneAppOp::ItemData as u16 {
                // OP_ItemData (0x5394) - Pipe-delimited item strings
                // FALLBACK: If we get item data, we are definitely in-zone
                if !got_new_zone {
                    println!("🗺️  ItemData (Op=0x5394) received - assuming zoned in.");
                    got_new_zone = true;
                }
                
                if !sent_client_ready {
                    let app_data = &payload[2..];
                    let text = String::from_utf8_lossy(app_data);
                    println!("📦 ItemData (Op=0x5394, {} bytes)", payload.len());
                    if text.len() > 64 {
                        println!("   Sample: {}", &text[..64]);
                    }
                }

                // Respond with OP_ReqNewZone if we haven't yet
                if !sent_req_new_zone {
                    zone_conn.send_app_packet(ZoneAppOp::ReqNewZone as u16, &[]).await?;
                    println!(">> Sent OP_ReqNewZone");
                    sent_req_new_zone = true;
                }

            } else if app_opcode == 0x0F47 || app_opcode == ZoneAppOp::ZoneSpawns as u16 { 
                // Proper Titanium SpawnStruct parsing using SpawnStruct::read()
                let app_data = &payload[2..];
                
                if app_opcode == ZoneAppOp::ZoneSpawns as u16 {
                    println!("👥 Received Bulk ZoneSpawns (Op=0x2e78, {} bytes, app_data={})", payload.len(), app_data.len());
                } else {
                    println!("🧑 Received NewSpawn (Op=0x0F47, {} bytes, app_data={})", payload.len(), app_data.len());
                }

                // Log raw first 64 bytes for debugging
                let preview_len = std::cmp::min(64, app_data.len());
                println!("   RAW[..{}]: {:?}", preview_len, &app_data[..preview_len]);

                let mut reader = Cursor::new(app_data);
                let mut spawn_count = 0;
                
                // Try to parse as long as there's reasonable data remaining
                // SpawnStruct::read() advances cursor by 388 bytes and handles bounds errors
                while (reader.position() as usize) + 80 <= app_data.len() {
                    match SpawnStruct::read(&mut reader) {
                        Ok(spawn_struct) => {
                            spawn_count += 1;
                            println!("🧑 Spawn #{}: id={}, name='{}', pos=({:.1}, {:.1}, {:.1}), lvl={}, race={}, class={}, npc={}",
                                spawn_count, spawn_struct.spawn_id, spawn_struct.name,
                                spawn_struct.x, spawn_struct.y, spawn_struct.z,
                                spawn_struct.level, spawn_struct.race, spawn_struct.class,
                                spawn_struct.is_npc);

                            // File logging
                            {
                                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("debug_spawns.log") {
                                    let _ = writeln!(file, "PARSED: ID: {}, Name: {}, Pos: ({:.2}, {:.2}, {:.2}), Lvl: {}, Race: {}, NPC: {}",
                                        spawn_struct.spawn_id, spawn_struct.name,
                                        spawn_struct.x, spawn_struct.y, spawn_struct.z,
                                        spawn_struct.level, spawn_struct.race, spawn_struct.is_npc);
                                }
                            }

                            if let Ok(mut state) = zone_state.lock() {
                                state.add_or_update_spawn(Spawn {
                                    id: spawn_struct.spawn_id,
                                    name: spawn_struct.name,
                                    level: spawn_struct.level,
                                    race: spawn_struct.race as u16,
                                    class: spawn_struct.class,
                                    hp: spawn_struct.cur_hp,
                                    max_hp: spawn_struct.max_hp,
                                    x: spawn_struct.x,
                                    y: spawn_struct.y,
                                    z: spawn_struct.z,
                                    heading: spawn_struct.heading,
                                    is_npc: spawn_struct.is_npc,
                                });
                                // Broadcast update immediately
                                if let Ok(json) = serde_json::to_string(&*state) {
                                    let _ = tx.send(json);
                                }
                            }
                        }
                        Err(e) => {
                            println!("⚠️ SpawnStruct parse error at offset {}/{}: {}", reader.position(), app_data.len(), e);
                            break;
                        }
                    }
                }
                
                if spawn_count == 0 && app_data.len() > 0 {
                    println!("⚠️ Got spawn packet ({} bytes) but parsed 0 spawns! Struct size might be wrong.", app_data.len());
                    
                    // AGGRESSIVE SCAN: Search for ME in the raw data
                    let mut state = zone_state.lock().unwrap();
                    let my_name = state.my_name.clone();
                    if !my_name.is_empty() && state.my_spawn_id.is_none() {
                        println!("🔍 Scanning raw data for char name: '{}'", my_name);
                        if let Some(pos) = app_data.windows(my_name.len()).position(|w| w.eq_ignore_ascii_case(my_name.as_bytes())) {
                            println!("🎯 FOUND name at offset {}!", pos);
                            // Titanium: Name at 7, SpawnID at 340
                            let base_offset = pos.saturating_sub(7);
                            if base_offset + 344 <= app_data.len() {
                                let id_bytes = &app_data[base_offset + 340 .. base_offset + 344];
                                let found_id = u32::from_le_bytes([id_bytes[0], id_bytes[1], id_bytes[2], id_bytes[3]]);
                                println!("✨ DISCOVERED My SpawnID: {} (via scan)", found_id);
                                state.my_spawn_id = Some(found_id);
                            }
                        }
                    }
                } else {
                    println!("   Parsed {} spawns from packet ({} bytes)", spawn_count, app_data.len());
                }

            } else if app_opcode == ZoneAppOp::SpawnDoor as u16 {
                // Door spawns — skip rendering, just log
                println!("🚪 Received SpawnDoor (Op=0x4c24, {} bytes) — skipping", payload.len());

            } else if app_opcode == ZoneAppOp::ZoneServerInfo as u16 {
                println!("� Received ZoneServerInfo (Op=0x61b6)");

            } else if app_opcode == ZoneAppOp::SendExpZonein as u16 {
                println!("� Received SendExpZonein");

            } else if app_opcode == 0x1FA1 { // OP_WorldObjectsSent (Titanium-era Live)
                println!("📦 Received WorldObjectsSent");
                got_world_objects_sent = true;

                // Once we have WorldObjectsSent, send OP_ReqClientSpawn if not already sent
                if !sent_req_client_spawn && got_player_profile && got_new_zone {
                    zone_conn.send_app_packet(ZoneAppOp::ReqClientSpawn as u16, &[]).await?;
                    println!(">> Sent OP_ReqClientSpawn");
                    sent_req_client_spawn = true;
                }



            } else if app_opcode == ZoneAppOp::SpawnAppearance as u16 {
                println!("👤 Received SpawnAppearance ({} bytes)", payload.len());

            } else if app_opcode == ZoneAppOp::SendAAStats as u16 {
                println!("📈 Received SendAAStats");

            } else if app_opcode == ZoneAppOp::SendAATable as u16 {
                println!("📊 Received SendAATable ({} bytes)", payload.len());

            } else if app_opcode == ZoneAppOp::ClientReady as u16 {
                println!("✅ Received ClientReady from server!");

            } else if app_opcode == 0x14cb { // OP_ClientUpdate
                // println!("📍 Received ClientUpdate");
                let mut reader = Cursor::new(&payload[2..]);
                if let Ok(update) = ClientUpdate::read(&mut reader) {
                     // Update spawn in state
                     let mut state = zone_state.lock().unwrap();
                     state.update_spawn_pos(update.spawn_id as u32, update.x, update.y, update.z, update.heading);
                     
                     // Broadcast update
                     if let Ok(json) = serde_json::to_string(&*state) {
                         let _ = tx.send(json);
                     }
                }
            } else {
                // println!("📩 Zone Packet: Op=0x{:04X}, Size={}", app_opcode, payload.len());
            }

            // State machine debug
            if !sent_client_ready {
                println!("   [STATE] profile={} new_zone={} req_new_zone={} req_spawn={} ready={}",
                    got_player_profile, got_new_zone, sent_req_new_zone, sent_req_client_spawn, sent_client_ready);
            }

            // Fallback logic for ClientReady matching original code...
            if !sent_req_client_spawn && got_player_profile && sent_req_new_zone {
                zone_conn.send_app_packet(ZoneAppOp::ReqClientSpawn as u16, &[]).await?;
                println!(">> Sent OP_ReqClientSpawn (triggered by packet flow)");
                sent_req_client_spawn = true;
            }

            // Send OP_ClientReady if needed (after spawns start flowing)
            if !sent_client_ready && got_player_profile && sent_req_client_spawn {
                zone_conn.send_app_packet(ZoneAppOp::ClientReady as u16, &[]).await?;
                println!(">> Sent OP_ClientReady — CONNECTION ESTABLISHED! 🎉");
                sent_client_ready = true;
            }
        }
    }

    println!("\n🏆 Zone connection complete! Character '{}' is now in-world.", char_name_for_zone);
    println!("   Press Enter to disconnect...");
    let _ = prompt(">").await;

    Ok(())
}
