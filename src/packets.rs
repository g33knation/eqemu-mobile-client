use binrw::{BinRead, BinWrite, io::Cursor, BinReaderExt, BinResult, Endian};
use std::io::{Read, Write}; // for read_exact
use async_recursion::async_recursion;

// ============================================================================
// Reliable UDP Protocol Structs (Derived from akk-stack/reliable_stream_structs.h)
// ============================================================================

/// **Reliable Protocol Opcodes** (Wire Level)
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum ReliableOp {
    Padding = 0x00,
    SessionRequest = 0x01,
    SessionResponse = 0x02,
    Combined = 0x03,
    SessionDisconnect = 0x05,
    KeepAlive = 0x06,
    Packet = 0x09,
    Fragment = 0x0D,
    Ack = 0x15,
    AppCombined = 0x19, // Used by Login Server instead of Combined sometimes?
    OutOfSession = 0x1D,
}

/// **Reliable Stream Header**
/// The first 2 bytes of *every* packet.
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ReliableStreamHeader {
    pub zero: u8,    // Always 0
    pub opcode: u8,  // ReliableOp
}

/// **Session Request (Client -> Server)**
/// OpCode: 0x01
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct ReliableStreamConnect {
    pub zero: u8,
    pub opcode: u8,
    pub protocol_version: u32,
    pub connect_code: u32,
    pub max_packet_size: u32,
}

/// **Session Response (Server -> Client)**
/// OpCode: 0x02
#[derive(BinRead, BinWrite, Debug)]
#[br(big)]
#[bw(big)]
pub struct ReliableStreamConnectReply {
    pub zero: u8,
    pub opcode: u8,
    pub connect_code: u32,
    pub encode_key: u32,
    pub crc_bytes: u8,
    pub encode_pass1: u8,
    pub encode_pass2: u8,
    pub max_packet_size: u32,
}

/// **Reliable Packet Header**
/// Used for OP_Packet (0x09), OP_Fragment (0x0D), OP_Ack (0x15)
/// [Zero: u8] [OpCode: u8] [Sequence: u16]
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ReliableStreamReliableHeader {
    pub zero: u8,
    pub opcode: u8,
    #[br(big)] #[bw(big)] pub sequence: u16, 
}

/// **Reliable Fragment Header**
/// OpCode: 0x0D
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ReliableFragmentHeader {
    // Standard Reliable Header
    pub zero: u8,
    pub opcode: u8,
    #[br(big)] #[bw(big)] pub sequence: u16, 
    
    // Fragment Specifics
    #[br(big)] #[bw(big)] pub total_size: u32,
}

// ============================================================================
// Application Layer (Login Protocol) - Payload of OP_Packet (0x09)
// ============================================================================

/// **Login Application Opcodes**
/// akk-stack/loginserver/client_manager.cpp: Titanium Opcodes
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u16)]
pub enum LoginAppOp {
    SessionReady = 0x0001,
    Login = 0x0002,
    ServerListRequest = 0x0004,
    PlayEverquestRequest = 0x000d,
    PlayEverquestResponse = 0x0021, // Titanium specific
    ChatMessage = 0x0016, // Used as "Handshake Reply" in akk-stack (LoginHandShakeReply)
    LoginAccepted = 0x0017,
    ServerListResponse = 0x0018,
}

/// **Login Base Message** (Header for App Packets)
/// login_types.h: LoginBaseMessage (10 bytes) - Packed
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct LoginBaseMessage {
    pub sequence: i32,     // 2: handshake, 3: login, 4: serverlist
    pub compressed: u8,    // bool -> u8
    pub encrypt_type: i8,  // 1: invert, 2: des
    pub unk3: i32,         // unused
}

/// **Login Handshake Reply** (Server -> Client)
/// OpCode: OP_ChatMessage (0x0016)
/// Wraps BaseMessage + BaseReply + unknown string
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct LoginHandShakeReply {
    pub base_header: LoginBaseMessage,
    pub success: u8,       // bool? LoginBaseReplyMessage
    pub error_str_id: i32,
    // Variable string follows (usually just null terminator or empty)
}

/// **Session Ready** (Client -> Server)
/// OpCode: OP_SessionReady (0x0001)
/// Only payload is a u32? No, `client.cpp` says:
/// `HandleSessionReady` reads `const char *data`.
/// Wait, `client.cpp` says: `if (size < sizeof(unsigned int)) LogError("Session ready was too small");`
/// But that might be checking the *application packet* size which likely includes OpCode?
/// Actually `EQApplicationPacket` usually has: [OpCode: u16] [Payload]
/// `client.cpp`: `HandleSessionReady((const char *) app->pBuffer, app->Size());`
/// Standard login protocol: SessionReady is just opcode + empty payload? Or OpCode + maybe a sequence?
/// Most implementations send OpCode 0x0001 and then 0-4 bytes. 
/// In `akk-stack`, `SessionReady` handler does not read the payload other than checking size. 
/// Let's assume we just send the OpCode packet for now.
#[derive(BinRead, BinWrite, Debug)]
pub struct SessionReady {
    // Empty
}

/// **Login Packet** (Client -> Server)
/// OpCode: OP_Login (0x0002)
/// Payload: [OpCode: u16] [LoginBaseMessage] [EncryptedBlock]
/// The `EncryptedBlock` is DES-CBC (Null Key) of: [username: String] [password: String]
/// `client.cpp`: `eqcrypt_block(..., false)` (Decrypt)
/// The payload structure being encrypted is just `user\0pass\0`?
/// `client.cpp`: `std::string user(&outbuffer[0]); ... cred = (&outbuffer[1 + user.length()]);`
/// Yes, it's null-terminated strings packed together.

/// **Player Login Reply** (Server -> Client)
/// Encrypted payload of OP_LoginAccepted (0x0017)
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct PlayerLoginReply {
    pub base_reply: LoginBaseReplyMessage,
    pub unk1: u8,
    pub unk2: u8,
    pub lsid: i32,
    pub key: [u8; 11], 
    pub unknown_pad: u8, // Padding for 8-byte alignment / struct diff?
    pub failed_attempts: i32,
    pub show_player_count: u32, // bool -> u32 (int in C++ usually 4 bytes)
    pub offer_min_days: i32,
    pub offer_min_views: i32,
    pub offer_cooldown_minutes: i32,
    pub web_offer_number: i32,
    pub web_offer_min_days: i32,
    pub web_offer_min_views: i32,
    pub web_offer_cooldown_minutes: i32,
    // Variable length username follows, but we can ignore it for now or rely on custom parser if needed.
    // For now, let's just parse the fixed part to get the key.
}

/// **Play Everquest Request** (Client -> Server)
/// OpCode: OP_PlayEverquestRequest (0x000d)
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct PlayEverquestRequest {
    pub base_header: LoginBaseMessage,
    pub server_number: u32,
}

/// **Play Everquest Response** (Server -> Client)
/// OpCode: OP_PlayEverquestResponse (0x0021)
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct PlayEverquestResponse {
    pub base_header: LoginBaseMessage,
    pub base_reply: LoginBaseReplyMessage,
    pub server_number: u32,
}

// ============================================================================
// Server List Structures
// ============================================================================

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ServerListRequest {
    pub unknown: u32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct LoginBaseReplyMessage {
    pub success: u8,
    pub error_str_id: i32,
    pub unknown: u8, // char str[1] in C++
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct LoginClientServerData {
    #[br(parse_with = read_null_term_string)]
    #[bw(write_with = write_null_term_string)]
    pub ip: String,
    
    pub server_type: i32,
    pub server_id: i32,
    
    #[br(parse_with = read_null_term_string)]
    #[bw(write_with = write_null_term_string)]
    pub server_name: String,
    
    #[br(parse_with = read_null_term_string)]
    #[bw(write_with = write_null_term_string)]
    pub country_code: String,
    
    #[br(parse_with = read_null_term_string)]
    #[bw(write_with = write_null_term_string)]
    pub language_code: String,
    
    pub server_status: i32,
    pub player_count: i32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ServerListReply {
    pub base_header: LoginBaseMessage,
    pub base_reply: LoginBaseReplyMessage,
    pub server_count: i32,
    // Array follows manually
    #[br(count = server_count)]
    pub servers: Vec<LoginClientServerData>,
}

// Logic for null Terminated Strings


fn read_null_term_string<R: Read + std::io::Seek>(reader: &mut R, _endian: Endian, _args: ()) -> BinResult<String> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        if byte[0] == 0 {
            break;
        }
        bytes.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn write_null_term_string<W: Write + std::io::Seek>(string: &String, writer: &mut W, _endian: Endian, _args: ()) -> BinResult<()> {
    writer.write_all(string.as_bytes())?;
    writer.write_all(&[0])?;
    Ok(())
}


#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum WorldAppOp {
    SendLoginInfo = 0x4dd0,
    ApproveWorld = 0x3c25,
    LogServer = 0x0fa6,
    SendCharInfo = 0x6957,
    SendCharInfoNew = 0x4513,
    EnterWorld = 0x7cba,
    PostEnterWorld = 0x52a4,
    ZoneEntry = 0x7213,
    ZoneServerInfo = 0x61b6,
}

// ============================================================================
// Zone Server Application Opcodes (RoF2)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ZoneAppOp {
    ZoneEntry = 0x7213,           // Client→Zone: identify character (Titanium)
    PlayerProfile = 0x75df,       // Server→Client: full player profile
    NewZone = 0x0920,             // Server→Client: binary zone metadata
    ItemData = 0x5394,            // Server→Client: pipe-delimited item data (previously misidentified as NewZone)
    ReqNewZone = 0x7ac5,          // Client→Server: request zone data
    ReqClientSpawn = 0x0322,      // Client→Server: request spawns/objects
    ClientReady = 0x345d,         // Client→Server: signal fully loaded (Guess, common)
    SendExpZonein = 0x0587,       // Server→Client: exp zone-in
    SpawnDoor = 0x4c24,           // Server→Client: door spawns
    ZoneSpawns = 0x2e78,          // Server→Client: NPC/player spawns
    TimeOfDay = 0x1580,           // Server→Client: world time
    SetServerFilter = 0x6563,     // Both
    SpawnAppearance = 0x7c32,     // Bidirectional: appearance updates (Titanium)
    SendAAStats = 0x5996,         // Server→Client: AA stats
    SendAATable = 0x367d,         // Both: AA table
    ZoneServerInfo = 0x61b6,      // Server→Client: IP/Port info
    Action = 0x497c,              // Animation / Spell Cast
    Damage = 0x5c78,              // Melee Damage
    AutoAttack = 0x5e55,          // Client→Server: Toggle Attack
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct CombatDamage {
    pub target: u16,
    pub source: u16,
    pub _type: u8,
    pub spell_id: u16,
    pub damage: u32,
    pub force: f32,
    pub hit_heading: f32,
    pub hit_pitch: f32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct Action {
    pub target: u16,
    pub source: u16,
    pub level: u16,
    pub instrument_mod: u32,
    pub force: f32,
    pub hit_heading: f32,
    pub hit_pitch: f32,
    pub _type: u8,
    pub unknown23: u16,
    pub unknown25: u16,
    pub spell: u16,
    pub spell_level: u8,
    pub effect_flag: u8,
}

// 332: /// **Client Zone Entry** (Client → Zone Server)
// 333: /// OpCode: OP_ZoneEntry (0x5089)
// 334: /// The client sends this to the zone server to identify itself.
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ClientZoneEntry {
    pub unknown00: u32,           // Titanium: 4 bytes of padding/unknown
    pub char_name: [u8; 64],      // Null-terminated character name
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct LoginInfo {
    #[br(count = 64)]
    pub login_info: Vec<u8>,
    #[br(count = 124)]
    pub unknown064: Vec<u8>,
    pub zoning: u8,
    #[br(count = 299)] // 488 - 189 = 299
    pub unknown189: Vec<u8>,
}

// --- Titanium Character Selection Structs ---

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct TintStruct {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub use_tint: u8,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct TintProfile {
    pub head: TintStruct,
    pub chest: TintStruct,
    pub arms: TintStruct,
    pub wrist: TintStruct,
    pub hands: TintStruct,
    pub legs: TintStruct,
    pub feet: TintStruct,
    pub primary: TintStruct,
    pub secondary: TintStruct,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct TextureStruct {
    pub material: u32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct TextureProfile {
    pub head: TextureStruct,
    pub chest: TextureStruct,
    pub arms: TextureStruct,
    pub wrist: TextureStruct,
    pub hands: TextureStruct,
    pub legs: TextureStruct,
    pub feet: TextureStruct,
    pub primary: TextureStruct,
    pub secondary: TextureStruct,
}

/// Titanium CharacterSelect_Struct - 1704 bytes total
/// Layout from titanium_structs.h
#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct TitaniumCharInfo {
    /*0000*/ pub race: [u32; 10],           // 40 bytes - Characters Race
    /*0040*/ pub cs_colors: [[u8; 36]; 10], // 360 bytes - TintProfile (9 x TintStruct @ 4 bytes each = 36 bytes per profile)
    /*0400*/ pub beard_color: [u8; 10],     // 10 bytes
    /*0410*/ pub hair_style: [u8; 10],      // 10 bytes
    /*0420*/ pub equip: [[u8; 36]; 10],     // 360 bytes - TextureProfile (9 x TextureStruct @ 4 bytes each = 36 bytes per profile)
    /*0780*/ pub secondary_id_file: [u32; 10], // 40 bytes
    /*0820*/ pub unknown820: [u8; 10],      // 10 bytes - 10x ff
    /*0830*/ pub unknown830: [u8; 2],       // 2 bytes - 2x 00
    /*0832*/ pub deity: [u32; 10],          // 40 bytes
    /*0872*/ pub go_home: [u8; 10],         // 10 bytes - 1=Go Home available
    /*0882*/ pub tutorial: [u8; 10],        // 10 bytes - 1=Tutorial available
    /*0892*/ pub beard: [u8; 10],           // 10 bytes
    /*0902*/ pub unknown902: [u8; 10],      // 10 bytes - 10x ff
    /*0912*/ pub primary_id_file: [u32; 10], // 40 bytes
    /*0952*/ pub hair_color: [u8; 10],      // 10 bytes
    /*0962*/ pub unknown0962: [u8; 2],      // 2 bytes - 2x 00
    /*0964*/ pub zone: [u32; 10],           // 40 bytes
    /*1004*/ pub class: [u8; 10],           // 10 bytes
    /*1014*/ pub face: [u8; 10],            // 10 bytes
    /*1024*/ pub names: [[u8; 64]; 10],     // 640 bytes - Characters Names
    /*1664*/ pub gender: [u8; 10],          // 10 bytes
    /*1674*/ pub eye_color1: [u8; 10],      // 10 bytes
    /*1684*/ pub eye_color2: [u8; 10],      // 10 bytes
    /*1694*/ pub level: [u8; 10],           // 10 bytes - Characters Levels
    /*1704*/ // Total: 1704 bytes
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct EnterWorldRequest {
    #[br(count = 64)]
    pub name: Vec<u8>,      // 64-byte null-terminated name
    pub tutorial: u32,
    pub return_home: u32,
}

#[derive(BinRead, BinWrite, Debug)]
#[br(little)]
#[bw(little)]
pub struct ZoneEntry {
    #[br(count = 64)]
    pub char_name: Vec<u8>, // 64-byte name
    #[br(count = 120)]
    pub unknown: Vec<u8>,   // Unknown padding or structure
    pub zone_short_name: [u8; 32], // e.g., "poknowledge"
    pub zone_id: u32, 
    pub unknown2: u32,
    pub ip: [u8; 20],       // IP Address string (e.g. "192.168.1.50")
    #[br(count = 28)]
    pub unknown3: Vec<u8>,
    pub port: u32, 
}

// ----------------------------------------------------------------
// PLAYER PROFILE (Partial Implementation)
// ----------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerProfile {
    pub name: String,
    pub level: u8,
    pub race: u32,
    pub class: u32,
    pub gender: u32,
    pub cur_hp: u32,
    pub mana: u32,
    pub str: u32,
    pub sta: u32,
    pub dex: u32,
    pub agi: u32,
    pub int: u32,
    pub wis: u32,
    pub cha: u32,
    pub exp: u32,
    // Inventory Blob (5120 bytes at offset 7444)
    // We will parse this later, for now just store raw
    #[serde(skip)] // Don't serialize blob directly
    pub inventory_blob: Vec<u8>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
}

impl PlayerProfile {
    pub fn read(reader: &mut Cursor<&[u8]>) -> BinResult<Self> {
        // Skip checksum(4)
        reader.set_position(4);
        
        let gender = reader.read_le::<u32>()?;
        let race = reader.read_le::<u32>()?;
        let class = reader.read_le::<u32>()?;
        
        // Level at offset 20
        reader.set_position(20);
        let level = reader.read_le::<u8>()?;
        
        // Stats:
        // 2228: mana
        // 2232: cur_hp
        // 2236: STR
        reader.set_position(2228);
        let mana = reader.read_le::<u32>()?;
        let cur_hp = reader.read_le::<u32>()?;
        let str = reader.read_le::<u32>()?;
        let sta = reader.read_le::<u32>()?;
        let cha = reader.read_le::<u32>()?;
        let dex = reader.read_le::<u32>()?;
        let int = reader.read_le::<u32>()?;
        let agi = reader.read_le::<u32>()?;
        let wis = reader.read_le::<u32>()?;

        // Inventory at 7444 (5120 bytes)
        reader.set_position(7444);
        let mut inventory_blob = vec![0u8; 5120];
        reader.read_exact(&mut inventory_blob)?;

        // Name at 12940
        reader.set_position(12940);
        let mut name_buf = [0u8; 64];
        reader.read_exact(&mut name_buf)?;
        let name = String::from_utf8_lossy(&name_buf).trim_matches('\0').to_string();

        // Exp at 13068
        reader.set_position(13068);
        let exp = reader.read_le::<u32>()?;
        
        // XYZ at 13116
        reader.set_position(13116);
        let x = reader.read_le::<f32>()?;
        let y = reader.read_le::<f32>()?;
        let z = reader.read_le::<f32>()?;
        let heading = reader.read_le::<f32>()?;

        Ok(PlayerProfile {
            name, 
            level,
            race,
            class,
            gender,
            cur_hp,
            mana,
            str, sta, dex, agi, int, wis, cha,
            exp,
            inventory_blob,
            x, y, z, heading,
        })
    }
}

// ----------------------------------------------------------------
// SPAWN STRUCT (Titanium)
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SpawnStruct {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub race: u32,
    pub class: u8,
    pub level: u8,
    pub cur_hp: u8,
    pub max_hp: u8,
    pub is_npc: bool,
    pub spawn_id: u32,
}

impl SpawnStruct {
    pub fn read(reader: &mut Cursor<&[u8]>) -> BinResult<Self> {
        let start_pos = reader.position();
        
        // 0x07: Name (64 bytes)
        reader.set_position(start_pos + 7);
        let mut name_buf = [0u8; 64];
        reader.read_exact(&mut name_buf)?;
        let name = String::from_utf8_lossy(&name_buf).trim_matches('\0').replace('_', " ").to_string();

        // 0x5E (94): Packed X / DeltaHeading (u32)
        // struct { deltaHeading:10; x:19; padding:3; }
        // x starts at bit 10.
        reader.set_position(start_pos + 94);
        let packed_x = reader.read_le::<u32>()?;
        let x_raw = (packed_x >> 10) & 0x7FFFF;
        let x = sign_extend(x_raw, 19) as f32 / 8.0;

        // 0x62 (98): Packed Y (u32)
        // struct { y:19; anim:10; padding:3; }
        // y starts at bit 0.
        let packed_y = reader.read_le::<u32>()?;
        let y_raw = packed_y & 0x7FFFF;
        let y = sign_extend(y_raw, 19) as f32 / 8.0;

        // 0x66 (102): Packed Z (u32)
        // struct { z:19; deltaY:13; }
        // z starts at bit 0.
        let packed_z = reader.read_le::<u32>()?;
        let z_raw = packed_z & 0x7FFFF;
        let z = sign_extend(z_raw, 19) as f32 / 8.0;

        // 0x6A (106): Packed Heading (u32)
        // struct { deltaX:13; heading:12; padding:7; }
        // heading starts at bit 13.
        let packed_h = reader.read_le::<u32>()?;
        let heading_raw = (packed_h >> 13) & 0xFFF;
        let heading = heading_raw as f32; 

        // 0x97 (151): Level
        reader.set_position(start_pos + 151);
        let level = reader.read_le::<u8>()?;

        // 0x11C (284): Race
        reader.set_position(start_pos + 284);
        let race = reader.read_le::<u32>()?;

        // 0x14B (331): Class
        reader.set_position(start_pos + 331);
        let class = reader.read_le::<u8>()?;
        
        // 0x154 (340): SpawnID
        reader.set_position(start_pos + 340);
        let spawn_id = reader.read_le::<u32>()?;

        // 0x90 (144): is_npc
        reader.set_position(start_pos + 144);
        let is_npc = reader.read_le::<u8>()? != 0;
        
        // HP
        reader.set_position(start_pos + 86);
        let cur_hp = reader.read_le::<u8>()?; 
        let max_hp = reader.read_le::<u8>()?;

        // Advance to next struct (approx 388 bytes, verify later)
        // For now, caller controls loop or we return size read?
        // Let's assume standard Titanium size 0x184 = 388 bytes?
        // Wait, line 338 in structs.h ends around offset 385.
        // Let's assume 388 bytes alignment.
        reader.set_position(start_pos + 388); 
        
        Ok(SpawnStruct {
            name,
            x, y, z, heading,
            race, class, level,
            cur_hp, max_hp,
            is_npc,
            spawn_id,
        })
    }
}

pub fn sign_extend(value: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    (value << shift) as i32 >> shift
}

// ----------------------------------------------------------------
// CLIENT UPDATE (Position Update)
// ----------------------------------------------------------------
#[derive(Debug)]
pub struct ClientUpdate {
    pub spawn_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
}

impl ClientUpdate {
    pub fn read(reader: &mut Cursor<&[u8]>) -> BinResult<Self> {
        let spawn_id = reader.read_le::<u16>()?;
        
        // Titanium PlayerPositionUpdateServer_Struct
        // 0x02: delta_heading:10, x_pos:19, padding:3
        let packed_1 = reader.read_le::<u32>()?;
        let x_raw = (packed_1 >> 10) & 0x7FFFF;
        let x = sign_extend(x_raw, 19) as f32 / 8.0;
        
        // 0x06: y_pos:19, animation:10, padding:3
        let packed_2 = reader.read_le::<u32>()?;
        let y_raw = packed_2 & 0x7FFFF;
        let y = sign_extend(y_raw, 19) as f32 / 8.0;
        
        // 0x10: z_pos:19, delta_y:13
        let packed_3 = reader.read_le::<u32>()?;
        let z_raw = packed_3 & 0x7FFFF;
        let z = sign_extend(z_raw, 19) as f32 / 8.0;
        
        // 0x14: delta_x:13, heading:12, padding:7
        let packed_4 = reader.read_le::<u32>()?;
        // Heading is 12 bits at offset 13
        let heading_raw = (packed_4 >> 13) & 0xFFF;
        let heading = heading_raw as f32; // Raw Titanium heading (0-4096 approx 512 units = 360/8?) 
        // Actually EQ heading: 0-512 = 360 degrees. 
        // Wait, Titanium might use 0-4096?
        // SpawnStruct heading was 0..4096 (12 bits). 
        // Standard EQ heading is 0..512. 
        // If it's 12 bits, it's likely 0..4096. 
        // We'll pass it raw to JS which handles conversion if needed.

        // 0x18: delta_z:13, padding:19
        let _packed_5 = reader.read_le::<u32>()?;
        
        Ok(ClientUpdate {
            spawn_id,
            x, y, z, heading,
        })
    }
}

#[derive(Debug, Default)]
pub struct ClientUpdateClient {
    pub spawn_id: u16,
    pub sequence: u16,
    pub y_pos: f32,
    pub delta_z: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub animation: u16,      // 10 bits
    pub delta_heading: u16,  // 10 bits
    pub x_pos: f32,
    pub z_pos: f32,
    pub heading: u16,        // 12 bits
}

impl ClientUpdateClient {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(36);
        buf.extend_from_slice(&self.spawn_id.to_le_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        
        // Pack positions as 19-bit signed integers (matching ClientUpdate::read() format)
        // Position values are multiplied by 8.0 before packing
        
        // Field at offset 4: delta_heading:10, x_pos:19, padding:3
        let x_packed = ((self.x_pos * 8.0) as i32) as u32 & 0x7FFFF;
        let dh_packed = self.delta_heading as u32 & 0x3FF;
        let packed_1 = dh_packed | (x_packed << 10);
        buf.extend_from_slice(&packed_1.to_le_bytes());
        
        // Field at offset 8: y_pos:19, animation:10, padding:3
        let y_packed = ((self.y_pos * 8.0) as i32) as u32 & 0x7FFFF;
        let anim_packed = self.animation as u32 & 0x3FF;
        let packed_2 = y_packed | (anim_packed << 19);
        buf.extend_from_slice(&packed_2.to_le_bytes());
        
        // Field at offset 12: z_pos:19, delta_y:13
        let z_packed = ((self.z_pos * 8.0) as i32) as u32 & 0x7FFFF;
        let dy_packed = ((self.delta_y * 8.0) as i32) as u32 & 0x1FFF;
        let packed_3 = z_packed | (dy_packed << 19);
        buf.extend_from_slice(&packed_3.to_le_bytes());
        
        // Field at offset 16: delta_x:13, heading:12, padding:7
        let dx_packed = ((self.delta_x * 8.0) as i32) as u32 & 0x1FFF;
        let h_packed = self.heading as u32 & 0xFFF;
        let packed_4 = dx_packed | (h_packed << 13);
        buf.extend_from_slice(&packed_4.to_le_bytes());
        
        // Field at offset 20: delta_z:13, padding:19
        let dz_packed = ((self.delta_z * 8.0) as i32) as u32 & 0x1FFF;
        buf.extend_from_slice(&dz_packed.to_le_bytes());
        
        buf
    }
}

// 0x184D: OP_TargetCommand
#[derive(Debug, Default)]
pub struct ClientTarget {
    pub target_id: u32,
}

impl ClientTarget {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.target_id.to_le_bytes().to_vec()
    }
}
