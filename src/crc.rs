use crc::{Algorithm, Crc};

/// ESP32 CRC32 implementation (`esp_rom_crc32_le`)
/// This has only been verified to be identical with one input-output pair so use with caution.
pub(crate) fn esp_crc32(bytes: &[u8; 4]) -> u32 {
    let buffer = bytes.map(|v| !v);
    !Crc::<u32>::new(&CRC_32_ESP).checksum(&buffer)
}

const CRC_32_ESP: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: u32::MAX,
    refin: true,
    refout: true,
    xorout: 0,
    check: 0,
    residue: 0,
};
