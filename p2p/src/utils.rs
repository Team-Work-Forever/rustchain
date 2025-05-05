pub fn to_32bytes(bytes: Vec<u8>) -> Option<[u8; 32]> {
    if bytes.len() != 32 {
        return None;
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Some(array)
}

pub fn to_64bytes(bytes: Vec<u8>) -> Option<[u8; 64]> {
    if bytes.len() != 64 {
        return None;
    }

    let mut array = [0u8; 64];
    array.copy_from_slice(&bytes);
    Some(array)
}
