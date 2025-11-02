/// Compute IPv4 header checksum
pub fn ipv4_checksum(_header: &[u8]) -> u16 {
    // RFC 791: ones'-complement sum of 16-bit words of the header with the
    // checksum field (bytes 10..12) treated as zero. Return the one's
    // complement of the final sum.
    if _header.len() < 20 {
        return 0;
    }

    let mut sum: u32 = 0;
    let len = _header.len();
    let mut i = 0usize;
    while i + 1 < len {
        // treat checksum field as zero
        if i == 10 {
            // add zero for checksum field
            i += 2;
            continue;
        }
        let word = u16::from_be_bytes([_header[i], _header[i+1]]) as u32;
        sum = sum.wrapping_add(word);
        i += 2;
    }
    // If odd length (shouldn't happen for IPv4 header) throw error
    if i < len {
        panic!("Invalid IPv4 header length");
    }

    // fold carries
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Compute UDP checksum
pub fn udp_checksum(_src: [u8;4], _dst: [u8;4], _udp: &[u8]) -> u16 {
    // Pseudo-header: src(4) + dst(4) + zero(1) + proto(1) + udp_len(2)
    // UDP checksum covers pseudo-header + UDP header + payload with the checksum
    // field (bytes 6..8 in the UDP header) treated as zero. Sum is ones'-complement.
    let udp_len = _udp.len() as u16;
    let mut sum: u32 = 0;

    // src
    for i in 0..2 {
        let word = u16::from_be_bytes([_src[i*2], _src[i*2+1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    // dst
    for i in 0..2 {
        let word = u16::from_be_bytes([_dst[i*2], _dst[i*2+1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    // zero + protocol (17 for UDP)
    sum = sum.wrapping_add(0u32);
    sum = sum.wrapping_add(17u32);
    // udp length
    sum = sum.wrapping_add(udp_len as u32);

    // UDP header + payload
    let mut i = 0usize;
    while i < _udp.len() {
        // treat checksum field (bytes 6..7) as zero
        if i == 6 {
            // add zero for checksum field
            i += 2;
            continue;
        }
        let word: u16 = if i + 1 < _udp.len() {
            u16::from_be_bytes([_udp[i], _udp[i+1]])
        } else {
            // odd-length: pad last byte with zero as low-order octet
            u16::from_be_bytes([_udp[i], 0u8])
        };
        sum = sum.wrapping_add(word as u32);
        i += 2;
    }

    // fold carries
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    let res = !(sum as u16);
    // Per RFC: transmit 0xFFFF if result is 0x0000
    if res == 0 { 0xffff } else { res }
}

/// Verify a UDP packet's checksum. Returns true if the checksum is valid or
/// if the checksum field is zero (no-checksum for IPv4). This function also
/// validates that the UDP length field is consistent with the provided buffer.
pub fn verify_udp_checksum(src: [u8;4], dst: [u8;4], udp: &[u8]) -> bool {
    if udp.len() < 8 { return false; }
    // Length field is at bytes 4..6
    let udp_len_field = u16::from_be_bytes([udp[4], udp[5]]) as usize;
    if udp_len_field < 8 { return false; }
    if udp.len() != udp_len_field { return false; }
    let ck_in = u16::from_be_bytes([udp[6], udp[7]]);
    // Per IPv4: checksum value 0 means "no checksum" and should be accepted
    if ck_in == 0 { return true; }
    let expected = udp_checksum(src, dst, udp);
    expected == ck_in
}

/// Compute ICMP/ICMP-like checksum (ones' complement) for the given buffer.
/// Returns the 16-bit checksum value to write into the packet.
pub fn compute_ck16(buf: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0usize;
    while i + 1 < buf.len() {
        let word = u16::from_be_bytes([buf[i], buf[i+1]]) as u32;
        sum = sum.wrapping_add(word);
        i += 2;
    }
    if i < buf.len() {
        // odd length: pad low byte with zero
        let word = u16::from_be_bytes([buf[i], 0u8]) as u32;
        sum = sum.wrapping_add(word);
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}