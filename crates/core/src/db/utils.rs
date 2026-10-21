pub fn get_offset(buf: &[u8], offset: usize, length: usize) -> &[u8] {
    &buf[offset..offset + length]
}

pub fn parse_varint(buf: &[u8]) -> (u64, usize) {
    // convert a slice of bytes to u64, big endian
    let mut output = 0u64;

    // Up to 9 bytes could be use
    for (i, _b) in buf.iter().enumerate().take(9) {
        let b = buf[i];
        if i == 8 {
            output = (output << 8) | b as u64;
            return (output, 9);
        };

        output = (output << 7) | (b & 0x7f) as u64;
        // Check bit at 0
        if b & 0x80 == 0 {
            return (output, i + 1);
        };
    }

    (output, 0)
}
