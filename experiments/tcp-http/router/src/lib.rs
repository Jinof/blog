use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

const ETHERNET_HEADER_LEN: usize = 14;
const IPV4_MIN_HEADER_LEN: usize = 20;
const ETHERTYPE_IPV4: u16 = 0x0800;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn octets(self) -> [u8; 6] {
        self.0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl FromStr for MacAddress {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value.split(':').collect::<Vec<_>>();
        if parts.len() != 6 {
            return Err(format!("invalid MAC address: {value}"));
        }
        let mut octets = [0_u8; 6];
        for (index, part) in parts.iter().enumerate() {
            octets[index] = u8::from_str_radix(part, 16)
                .map_err(|_| format!("invalid MAC address: {value}"))?;
        }
        Ok(Self(octets))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ipv4Subnet {
    network: u32,
    mask: u32,
    prefix: u8,
}

impl Ipv4Subnet {
    pub fn contains(self, address: Ipv4Addr) -> bool {
        u32::from(address) & self.mask == self.network
    }
}

impl fmt::Display for Ipv4Subnet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}",
            Ipv4Addr::from(self.network),
            self.prefix
        )
    }
}

impl FromStr for Ipv4Subnet {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = value
            .split_once('/')
            .ok_or_else(|| format!("subnet must use CIDR notation: {value}"))?;
        let address = address
            .parse::<Ipv4Addr>()
            .map_err(|_| format!("invalid IPv4 subnet: {value}"))?;
        let prefix = prefix
            .parse::<u8>()
            .map_err(|_| format!("invalid IPv4 prefix: {value}"))?;
        if prefix > 32 {
            return Err(format!("IPv4 prefix must be between 0 and 32: {value}"));
        }
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        Ok(Self {
            network: u32::from(address) & mask,
            mask,
            prefix,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacketDropReason {
    NonIpv4,
    InvalidIpv4,
    RouteMiss,
    TtlExpired,
}

/// Prepares one Ethernet/IPv4 frame for the next routed hop.
///
/// TCP and UDP payloads are left untouched. Only the Ethernet addresses, IPv4
/// TTL, and IPv4 header checksum change, so the end-to-end TCP connection is
/// preserved.
pub fn prepare_ipv4_frame(
    frame: &mut Vec<u8>,
    destination_subnet: Ipv4Subnet,
    egress_mac: MacAddress,
    next_hop_mac: MacAddress,
) -> Result<(), PacketDropReason> {
    if frame.len() < ETHERNET_HEADER_LEN + IPV4_MIN_HEADER_LEN {
        return Err(PacketDropReason::InvalidIpv4);
    }
    if u16::from_be_bytes([frame[12], frame[13]]) != ETHERTYPE_IPV4 {
        return Err(PacketDropReason::NonIpv4);
    }

    let ip_start = ETHERNET_HEADER_LEN;
    let version = frame[ip_start] >> 4;
    let header_len = usize::from(frame[ip_start] & 0x0f) * 4;
    if version != 4 || header_len < IPV4_MIN_HEADER_LEN || frame.len() < ip_start + header_len {
        return Err(PacketDropReason::InvalidIpv4);
    }

    let total_len = usize::from(u16::from_be_bytes([
        frame[ip_start + 2],
        frame[ip_start + 3],
    ]));
    if total_len < header_len || frame.len() < ip_start + total_len {
        return Err(PacketDropReason::InvalidIpv4);
    }
    if ipv4_checksum(&frame[ip_start..ip_start + header_len]) != 0 {
        return Err(PacketDropReason::InvalidIpv4);
    }

    let destination = Ipv4Addr::new(
        frame[ip_start + 16],
        frame[ip_start + 17],
        frame[ip_start + 18],
        frame[ip_start + 19],
    );
    if !destination_subnet.contains(destination) {
        return Err(PacketDropReason::RouteMiss);
    }
    if frame[ip_start + 8] <= 1 {
        return Err(PacketDropReason::TtlExpired);
    }

    frame[ip_start + 8] -= 1;
    frame[ip_start + 10] = 0;
    frame[ip_start + 11] = 0;
    let checksum = ipv4_checksum(&frame[ip_start..ip_start + header_len]);
    frame[ip_start + 10..ip_start + 12].copy_from_slice(&checksum.to_be_bytes());
    frame[0..6].copy_from_slice(&next_hop_mac.octets());
    frame[6..12].copy_from_slice(&egress_mac.octets());
    frame.truncate(ip_start + total_len);
    Ok(())
}

fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum = 0_u32;
    let mut chunks = header.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u32::from(u16::from_be_bytes([chunk[0], chunk[1]]));
    }
    if let Some(byte) = chunks.remainder().first() {
        sum += u32::from(*byte) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[derive(Clone, Copy, Debug)]
pub struct LinkClock {
    next_link_free: Duration,
}

impl Default for LinkClock {
    fn default() -> Self {
        Self {
            next_link_free: Duration::ZERO,
        }
    }
}

impl LinkClock {
    /// Returns the delivery time relative to a shared epoch. Packet bytes are
    /// serialized at the configured link rate before propagation delay is
    /// added. Consecutive packets therefore build a real FIFO backlog.
    pub fn schedule(
        &mut self,
        arrival: Duration,
        frame_bytes: usize,
        rate_bps: f64,
        propagation_delay: Duration,
    ) -> Duration {
        let transmission_start = self.next_link_free.max(arrival);
        let serialization_seconds = frame_bytes as f64 * 8.0 / rate_bps;
        let serialization = Duration::from_secs_f64(serialization_seconds.max(1e-9));
        self.next_link_free = transmission_start + serialization;
        self.next_link_free + propagation_delay
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
struct XorShift64 {
    state: u64,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn should_drop(&mut self, loss_fraction: f64) -> bool {
        if loss_fraction <= 0.0 {
            return false;
        }
        if loss_fraction >= 1.0 {
            return true;
        }
        let sample = self.next_u64() as f64 / u64::MAX as f64;
        sample < loss_fraction
    }
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::run_from_env;

#[cfg(not(target_os = "linux"))]
pub fn run_from_env() -> Result<(), String> {
    Err(
        "raw packet forwarding requires Linux; run this binary inside the supplied Docker topology"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ipv4_frame(destination: Ipv4Addr, ttl: u8) -> Vec<u8> {
        let tcp_header_len = 20;
        let ip_total_len = IPV4_MIN_HEADER_LEN + tcp_header_len;
        let mut frame = vec![0_u8; ETHERNET_HEADER_LEN + ip_total_len];
        frame[0..6].copy_from_slice(&[0, 1, 2, 3, 4, 5]);
        frame[6..12].copy_from_slice(&[6, 7, 8, 9, 10, 11]);
        frame[12..14].copy_from_slice(&ETHERTYPE_IPV4.to_be_bytes());
        frame[14] = 0x45;
        frame[16..18].copy_from_slice(&(ip_total_len as u16).to_be_bytes());
        frame[22] = ttl;
        frame[23] = 6;
        frame[26..30].copy_from_slice(&Ipv4Addr::new(172, 28, 0, 2).octets());
        frame[30..34].copy_from_slice(&destination.octets());
        for (offset, byte) in frame[34..].iter_mut().enumerate() {
            *byte = (offset as u8).wrapping_mul(7).wrapping_add(3);
        }
        let checksum = ipv4_checksum(&frame[14..34]);
        frame[24..26].copy_from_slice(&checksum.to_be_bytes());
        frame
    }

    #[test]
    fn routes_ipv4_without_touching_transport_payload() {
        let subnet = "172.29.0.0/24".parse::<Ipv4Subnet>().unwrap();
        let source_mac = "02:00:00:00:1d:fe".parse::<MacAddress>().unwrap();
        let next_hop = "02:00:00:00:1d:02".parse::<MacAddress>().unwrap();
        let mut frame = ipv4_frame(Ipv4Addr::new(172, 29, 0, 2), 64);
        let transport_before = frame[34..].to_vec();

        prepare_ipv4_frame(&mut frame, subnet, source_mac, next_hop).unwrap();

        assert_eq!(&frame[0..6], &next_hop.octets());
        assert_eq!(&frame[6..12], &source_mac.octets());
        assert_eq!(frame[22], 63);
        assert_eq!(ipv4_checksum(&frame[14..34]), 0);
        assert_eq!(frame[23], 6);
        assert_eq!(&frame[34..], transport_before.as_slice());
    }

    #[test]
    fn rejects_route_misses_and_expired_ttl() {
        let subnet = "172.29.0.0/24".parse::<Ipv4Subnet>().unwrap();
        let mac = "02:00:00:00:00:01".parse::<MacAddress>().unwrap();
        let mut wrong_route = ipv4_frame(Ipv4Addr::new(192, 0, 2, 1), 64);
        let mut expired = ipv4_frame(Ipv4Addr::new(172, 29, 0, 2), 1);

        assert_eq!(
            prepare_ipv4_frame(&mut wrong_route, subnet, mac, mac),
            Err(PacketDropReason::RouteMiss)
        );
        assert_eq!(
            prepare_ipv4_frame(&mut expired, subnet, mac, mac),
            Err(PacketDropReason::TtlExpired)
        );
    }

    #[test]
    fn serializes_packets_before_adding_propagation_delay() {
        let mut clock = LinkClock::default();
        let delay = Duration::from_millis(14);
        let first = clock.schedule(Duration::ZERO, 1000, 8_000.0, delay);
        let second = clock.schedule(Duration::ZERO, 1000, 8_000.0, delay);

        assert_eq!(first, Duration::from_millis(1014));
        assert_eq!(second, Duration::from_millis(2014));
    }

    #[test]
    fn parses_and_normalizes_subnets() {
        let subnet = "172.29.0.123/24".parse::<Ipv4Subnet>().unwrap();
        assert_eq!(subnet.to_string(), "172.29.0.0/24");
        assert!(subnet.contains(Ipv4Addr::new(172, 29, 0, 2)));
        assert!(!subnet.contains(Ipv4Addr::new(172, 28, 0, 2)));
    }

    #[test]
    fn seeded_loss_is_reproducible() {
        let mut first = XorShift64::new(42);
        let mut second = XorShift64::new(42);
        let first_samples = (0..128).map(|_| first.should_drop(0.1)).collect::<Vec<_>>();
        let second_samples = (0..128)
            .map(|_| second.should_drop(0.1))
            .collect::<Vec<_>>();
        assert_eq!(first_samples, second_samples);
        assert!(first_samples.iter().any(|sample| *sample));
        assert!(first_samples.iter().any(|sample| !*sample));
    }
}
