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
    (!(sum as u16))
}

/// Compute UDP checksum (stub)
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
    while i + 1 < _udp.len() {
        // treat checksum field (bytes 6..7) as zero
        if i == 6 {
            // add zero for checksum field
            i += 2;
            continue;
        }
        let word = u16::from_be_bytes([_udp[i], _udp[i+1]]) as u32;
        sum = sum.wrapping_add(word);
        i += 2;
    }
    if i < _udp.len() {
        // odd-length: panic
        panic!("Invalid UDP packet length");
    }

    // fold carries
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    let res = !(sum as u16);
    // Per RFC: transmit 0xFFFF if result is 0x0000
    if res == 0 { 0xffff } else { res }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checksum_types() {
        let _ = ipv4_checksum(&[0u8; 20]);
        let _ = udp_checksum([0,0,0,0], [0,0,0,0], &[]);
    }

    #[test]
    fn ipv4_checksum_roundtrip() {
        // construct a minimal IPv4 header with zeros in checksum field
        let mut hdr = [0u8; 20];
        hdr[0] = 0x45; // version=4, ihl=5
        hdr[1] = 0; // tos
        hdr[2..4].copy_from_slice(&0u16.to_be_bytes()); // total len
        hdr[4..6].copy_from_slice(&0u16.to_be_bytes()); // id
        hdr[6..8].copy_from_slice(&0u16.to_be_bytes()); // flags/frag
        hdr[8] = 64; // ttl
        hdr[9] = 6; // proto TCP
        hdr[10] = 0; hdr[11] = 0; // checksum zero
        hdr[12..16].copy_from_slice(&[192,168,0,1]);
        hdr[16..20].copy_from_slice(&[192,168,0,2]);

        let c = ipv4_checksum(&hdr);
        hdr[10..12].copy_from_slice(&c.to_be_bytes());

        // raw sum including checksum should be 0xffff
        let mut sum: u32 = 0;
        for i in (0..hdr.len()).step_by(2) {
            let word = u16::from_be_bytes([hdr[i], hdr[i+1]]) as u32;
            sum = sum.wrapping_add(word);
        }
        while (sum >> 16) != 0 { sum = (sum & 0xffff) + (sum >> 16); }
        assert_eq!(sum as u16, 0xffff);
    }

    #[test]
    fn udp_checksum_roundtrip() {
        let src = [10,0,0,1];
        let dst = [10,0,0,2];
        // UDP header: src port, dst port, len, checksum
        let mut udp = Vec::new();
        udp.extend_from_slice(&1234u16.to_be_bytes());
        udp.extend_from_slice(&4321u16.to_be_bytes());
        // placeholder length
        let payload = b"hello";
        let udp_len = (8 + payload.len()) as u16;
        udp.extend_from_slice(&udp_len.to_be_bytes());
        udp.extend_from_slice(&0u16.to_be_bytes()); // checksum zero
        udp.extend_from_slice(payload);

        let c = udp_checksum(src, dst, &udp);
        // write checksum into packet
        udp[6..8].copy_from_slice(&c.to_be_bytes());

        // compute raw sum over pseudo-header + udp packet, should be 0xffff
        let mut sum: u32 = 0;
        // src
        for i in 0..2 { sum = sum.wrapping_add(u16::from_be_bytes([src[i*2], src[i*2+1]]) as u32); }
        // dst
        for i in 0..2 { sum = sum.wrapping_add(u16::from_be_bytes([dst[i*2], dst[i*2+1]]) as u32); }
        // zero + proto
        sum = sum.wrapping_add(0);
        sum = sum.wrapping_add(17);
        sum = sum.wrapping_add(udp_len as u32);

        for i in (0..udp.len()).step_by(2) {
            let hi = udp[i];
            let lo = if i+1 < udp.len() { udp[i+1] } else { 0 };
            sum = sum.wrapping_add(u16::from_be_bytes([hi, lo]) as u32);
        }
        while (sum >> 16) != 0 { sum = (sum & 0xffff) + (sum >> 16); }
        assert_eq!(sum as u16, 0xffff);
    }
}
