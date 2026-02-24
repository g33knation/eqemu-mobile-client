use des::cipher::{
    BlockEncryptMut, BlockDecryptMut, KeyIvInit
};
use des::Des;

type DesCbcEnc = cbc::Encryptor<Des>;
type DesCbcDec = cbc::Decryptor<Des>;

/// Encrypts data using DES-CBC with a zeroed key and IV.
/// Mirrors EQEmu's eqcrypt_block (Zero padding to 8-byte blocks).
pub fn encrypt_null_des(data: &[u8]) -> Vec<u8> {
    let key = [0u8; 8];
    let iv = [0u8; 8];
    
    // Calculate required blocks (multiple of 8)
    let padded_len = if data.len() % 8 == 0 {
        data.len()
    } else {
        ((data.len() / 8) + 1) * 8
    };

    let mut buffer = vec![0u8; padded_len];
    buffer[..data.len()].copy_from_slice(data);
    
    use des::cipher::block_padding::NoPadding;
    let encryptor = DesCbcEnc::new(&key.into(), &iv.into());
    encryptor.encrypt_padded_mut::<NoPadding>(&mut buffer, data.len()).ok();
    
    buffer
}

/// Decrypts data using DES-CBC with a zeroed key and IV.
pub fn decrypt_null_des(data: &[u8]) -> Vec<u8> {
    let key = [0u8; 8];
    let iv = [0u8; 8];
    
    let mut buffer = data.to_vec();
    if buffer.len() % 8 != 0 {
        return Vec::new(); // Invalid size
    }

    let decryptor = DesCbcDec::new(&key.into(), &iv.into());
    // Note: cbc::Decryptor doesn't expose easy block-by-block with IV update manually as nicely as Pkcs7 wrapper,
    // but we can just use decrypt_padded_mut with NoPadding which is basically raw CBC.
    use des::cipher::block_padding::NoPadding;
    decryptor.decrypt_padded_mut::<NoPadding>(&mut buffer).ok();
    
    buffer
}
