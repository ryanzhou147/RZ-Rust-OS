/// Compute IPv4 header checksum (stub)
pub fn ipv4_checksum(_header: &[u8]) -> u16 {
    // TODO: implement real checksum
    0
}

/// Compute UDP checksum (stub)
pub fn udp_checksum(_src: [u8;4], _dst: [u8;4], _udp: &[u8]) -> u16 {
    // TODO: implement pseudo-header + UDP checksum
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checksum_types() {
        let _ = ipv4_checksum(&[0u8; 20]);
        let _ = udp_checksum([0,0,0,0], [0,0,0,0], &[]);
    }
}
