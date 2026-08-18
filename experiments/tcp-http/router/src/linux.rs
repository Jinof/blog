use super::{prepare_ipv4_frame, Ipv4Subnet, LinkClock, MacAddress, PacketDropReason, XorShift64};
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::fmt;
use std::fs;
use std::io;
use std::mem;
use std::os::raw::c_int;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const AF_PACKET: c_int = 17;
const SOCK_RAW: c_int = 3;
const ETH_P_IP: u16 = 0x0800;
const PACKET_OUTGOING: u8 = 4;
const PACKET_LOOPBACK: u8 = 5;
const SOL_SOCKET: c_int = 1;
const SO_RCVBUF: c_int = 8;
const SO_RCVBUFFORCE: c_int = 33;
const SOL_PACKET: c_int = 263;
const PACKET_STATISTICS: c_int = 6;
const RECEIVE_BUFFER_BYTES: c_int = 16 * 1024 * 1024;
const MAX_FRAME_LEN: usize = 65_536;

#[repr(C)]
struct SockAddrLl {
    sll_family: u16,
    sll_protocol: u16,
    sll_ifindex: i32,
    sll_hatype: u16,
    sll_pkttype: u8,
    sll_halen: u8,
    sll_addr: [u8; 8],
}

#[repr(C)]
#[derive(Default)]
struct TpacketStats {
    packets: u32,
    drops: u32,
}

unsafe extern "C" {
    fn socket(domain: c_int, socket_type: c_int, protocol: c_int) -> c_int;
    fn bind(socket: c_int, address: *const c_void, address_len: u32) -> c_int;
    fn recvfrom(
        socket: c_int,
        buffer: *mut c_void,
        length: usize,
        flags: c_int,
        address: *mut c_void,
        address_len: *mut u32,
    ) -> isize;
    fn sendto(
        socket: c_int,
        buffer: *const c_void,
        length: usize,
        flags: c_int,
        destination: *const c_void,
        destination_len: u32,
    ) -> isize;
    fn close(file_descriptor: c_int) -> c_int;
    fn setsockopt(
        socket: c_int,
        level: c_int,
        option_name: c_int,
        option_value: *const c_void,
        option_length: u32,
    ) -> c_int;
    fn getsockopt(
        socket: c_int,
        level: c_int,
        option_name: c_int,
        option_value: *mut c_void,
        option_length: *mut u32,
    ) -> c_int;
}

#[derive(Clone, Copy, Debug)]
struct LinkConfig {
    delay: Duration,
    rate_bps: f64,
    loss_fraction: f64,
    queue_bytes: u64,
}

#[derive(Debug)]
struct Config {
    left_interface: String,
    right_interface: String,
    left_subnet: Ipv4Subnet,
    right_subnet: Ipv4Subnet,
    left_next_hop_mac: MacAddress,
    right_next_hop_mac: MacAddress,
    left_to_right: LinkConfig,
    right_to_left: LinkConfig,
    seed: u64,
    metrics_interval: Duration,
}

#[derive(Default)]
struct DirectionStats {
    received_packets: AtomicU64,
    received_bytes: AtomicU64,
    forwarded_packets: AtomicU64,
    forwarded_bytes: AtomicU64,
    random_drops: AtomicU64,
    queue_drops: AtomicU64,
    channel_drops: AtomicU64,
    kernel_socket_drops: AtomicU64,
    invalid_drops: AtomicU64,
    route_misses: AtomicU64,
    ttl_drops: AtomicU64,
    send_errors: AtomicU64,
    peak_queue_bytes: AtomicU64,
}

impl DirectionStats {
    fn record_prepare_drop(&self, reason: PacketDropReason) {
        match reason {
            PacketDropReason::RouteMiss => {
                self.route_misses.fetch_add(1, Ordering::Relaxed);
            }
            PacketDropReason::TtlExpired => {
                self.ttl_drops.fetch_add(1, Ordering::Relaxed);
            }
            PacketDropReason::NonIpv4 | PacketDropReason::InvalidIpv4 => {
                self.invalid_drops.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn snapshot(&self, queued_bytes: u64) -> String {
        format!(
            concat!(
                "{{\"rx_packets\":{},\"rx_bytes\":{},",
                "\"forwarded_packets\":{},\"forwarded_bytes\":{},",
                "\"random_drops\":{},\"queue_drops\":{},",
                "\"channel_drops\":{},\"kernel_socket_drops\":{},",
                "\"invalid_drops\":{},",
                "\"route_misses\":{},\"ttl_drops\":{},",
                "\"send_errors\":{},\"queued_bytes\":{},",
                "\"peak_queue_bytes\":{}}}"
            ),
            self.received_packets.load(Ordering::Relaxed),
            self.received_bytes.load(Ordering::Relaxed),
            self.forwarded_packets.load(Ordering::Relaxed),
            self.forwarded_bytes.load(Ordering::Relaxed),
            self.random_drops.load(Ordering::Relaxed),
            self.queue_drops.load(Ordering::Relaxed),
            self.channel_drops.load(Ordering::Relaxed),
            self.kernel_socket_drops.load(Ordering::Relaxed),
            self.invalid_drops.load(Ordering::Relaxed),
            self.route_misses.load(Ordering::Relaxed),
            self.ttl_drops.load(Ordering::Relaxed),
            self.send_errors.load(Ordering::Relaxed),
            queued_bytes,
            self.peak_queue_bytes.load(Ordering::Relaxed),
        )
    }
}

struct RawPacketSocket {
    file_descriptor: c_int,
    interface_index: i32,
    interface_name: String,
    mac_address: MacAddress,
    receive_buffer_bytes: c_int,
}

unsafe impl Send for RawPacketSocket {}
unsafe impl Sync for RawPacketSocket {}

impl RawPacketSocket {
    fn open(interface_name: &str) -> io::Result<Self> {
        let interface_index = read_interface_index(interface_name)?;
        let mac_address = read_interface_mac(interface_name)?;
        let protocol = ETH_P_IP.to_be();
        let file_descriptor = unsafe { socket(AF_PACKET, SOCK_RAW, i32::from(protocol)) };
        if file_descriptor < 0 {
            return Err(io::Error::last_os_error());
        }

        if set_receive_buffer(file_descriptor, RECEIVE_BUFFER_BYTES) != 0 {
            let error = io::Error::last_os_error();
            unsafe {
                close(file_descriptor);
            }
            return Err(error);
        }
        let receive_buffer_bytes = match get_receive_buffer(file_descriptor) {
            Ok(bytes) => bytes,
            Err(error) => {
                unsafe {
                    close(file_descriptor);
                }
                return Err(error);
            }
        };

        let address = SockAddrLl {
            sll_family: AF_PACKET as u16,
            sll_protocol: protocol,
            sll_ifindex: interface_index,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 0,
            sll_addr: [0; 8],
        };
        let bind_result = unsafe {
            bind(
                file_descriptor,
                &address as *const SockAddrLl as *const c_void,
                mem::size_of::<SockAddrLl>() as u32,
            )
        };
        if bind_result != 0 {
            let error = io::Error::last_os_error();
            unsafe {
                close(file_descriptor);
            }
            return Err(error);
        }

        Ok(Self {
            file_descriptor,
            interface_index,
            interface_name: interface_name.to_string(),
            mac_address,
            receive_buffer_bytes,
        })
    }

    fn receive(&self, buffer: &mut [u8]) -> io::Result<(usize, u8)> {
        let mut source = SockAddrLl {
            sll_family: 0,
            sll_protocol: 0,
            sll_ifindex: 0,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 0,
            sll_addr: [0; 8],
        };
        let mut source_len = mem::size_of::<SockAddrLl>() as u32;
        let received = unsafe {
            recvfrom(
                self.file_descriptor,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                0,
                &mut source as *mut SockAddrLl as *mut c_void,
                &mut source_len,
            )
        };
        if received < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((received as usize, source.sll_pkttype))
    }

    fn send(&self, frame: &[u8]) -> io::Result<()> {
        if frame.len() < 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Ethernet frame is shorter than its destination address",
            ));
        }
        let mut destination_mac = [0_u8; 8];
        destination_mac[..6].copy_from_slice(&frame[..6]);
        let destination = SockAddrLl {
            sll_family: AF_PACKET as u16,
            sll_protocol: ETH_P_IP.to_be(),
            sll_ifindex: self.interface_index,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: destination_mac,
        };
        let sent = unsafe {
            sendto(
                self.file_descriptor,
                frame.as_ptr() as *const c_void,
                frame.len(),
                0,
                &destination as *const SockAddrLl as *const c_void,
                mem::size_of::<SockAddrLl>() as u32,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        if sent as usize != frame.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("only sent {sent} of {} frame bytes", frame.len()),
            ));
        }
        Ok(())
    }

    fn take_kernel_drops(&self) -> io::Result<u64> {
        let mut statistics = TpacketStats::default();
        let mut length = mem::size_of::<TpacketStats>() as u32;
        let result = unsafe {
            getsockopt(
                self.file_descriptor,
                SOL_PACKET,
                PACKET_STATISTICS,
                &mut statistics as *mut TpacketStats as *mut c_void,
                &mut length,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(u64::from(statistics.drops))
    }
}

fn set_receive_buffer(file_descriptor: c_int, bytes: c_int) -> c_int {
    let value = &bytes as *const c_int as *const c_void;
    let length = mem::size_of::<c_int>() as u32;
    let forced = unsafe { setsockopt(file_descriptor, SOL_SOCKET, SO_RCVBUFFORCE, value, length) };
    if forced == 0 {
        return 0;
    }
    unsafe { setsockopt(file_descriptor, SOL_SOCKET, SO_RCVBUF, value, length) }
}

fn get_receive_buffer(file_descriptor: c_int) -> io::Result<c_int> {
    let mut value = 0 as c_int;
    let mut length = mem::size_of::<c_int>() as u32;
    let result = unsafe {
        getsockopt(
            file_descriptor,
            SOL_SOCKET,
            SO_RCVBUF,
            &mut value as *mut c_int as *mut c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}

impl Drop for RawPacketSocket {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}

struct QueuedFrame {
    bytes: Vec<u8>,
    arrival: Instant,
}

struct ScheduledFrame {
    bytes: Vec<u8>,
    serialization_complete: Instant,
    delivery: Instant,
    queue_accounted: bool,
}

struct ReceivePath {
    destination_subnet: Ipv4Subnet,
    egress_mac: MacAddress,
    next_hop_mac: MacAddress,
    link: LinkConfig,
    seed: u64,
    queue: Sender<QueuedFrame>,
    reserved_queue_bytes: Arc<AtomicU64>,
    stats: Arc<DirectionStats>,
}

pub fn run_from_env() -> Result<(), String> {
    if std::env::args().any(|argument| argument == "--help" || argument == "-h") {
        print_help();
        return Ok(());
    }
    let config = parse_config()?;
    let left_socket = Arc::new(
        RawPacketSocket::open(&config.left_interface)
            .map_err(|error| format!("open {}: {error}", config.left_interface))?,
    );
    let right_socket = Arc::new(
        RawPacketSocket::open(&config.right_interface)
            .map_err(|error| format!("open {}: {error}", config.right_interface))?,
    );

    eprintln!(
        concat!(
            "lab-router: {} ({}, rcvbuf={}B) {} <-> {} {} ({}, rcvbuf={}B)\n",
            "lab-router: left-to-right delay={:?} rate={:.3}Mbit/s loss={:.5}% queue={}B\n",
            "lab-router: right-to-left delay={:?} rate={:.3}Mbit/s loss={:.5}% queue={}B"
        ),
        left_socket.interface_name,
        left_socket.mac_address,
        left_socket.receive_buffer_bytes,
        config.left_subnet,
        config.right_subnet,
        right_socket.interface_name,
        right_socket.mac_address,
        right_socket.receive_buffer_bytes,
        config.left_to_right.delay,
        config.left_to_right.rate_bps / 1e6,
        config.left_to_right.loss_fraction * 100.0,
        config.left_to_right.queue_bytes,
        config.right_to_left.delay,
        config.right_to_left.rate_bps / 1e6,
        config.right_to_left.loss_fraction * 100.0,
        config.right_to_left.queue_bytes,
    );

    let left_to_right_stats = Arc::new(DirectionStats::default());
    let right_to_left_stats = Arc::new(DirectionStats::default());
    let left_to_right_reserved = Arc::new(AtomicU64::new(0));
    let right_to_left_reserved = Arc::new(AtomicU64::new(0));
    let (left_to_right_sender, left_to_right_receiver) = mpsc::channel();
    let (right_to_left_sender, right_to_left_receiver) = mpsc::channel();
    let epoch = Instant::now();

    let left_scheduler_socket = Arc::clone(&right_socket);
    let left_scheduler_stats = Arc::clone(&left_to_right_stats);
    let left_scheduler_reserved = Arc::clone(&left_to_right_reserved);
    let left_link = config.left_to_right;
    thread::spawn(move || {
        scheduler_loop(
            left_to_right_receiver,
            left_scheduler_socket,
            left_link,
            left_scheduler_reserved,
            left_scheduler_stats,
            epoch,
        )
    });

    let right_scheduler_socket = Arc::clone(&left_socket);
    let right_scheduler_stats = Arc::clone(&right_to_left_stats);
    let right_scheduler_reserved = Arc::clone(&right_to_left_reserved);
    let right_link = config.right_to_left;
    thread::spawn(move || {
        scheduler_loop(
            right_to_left_receiver,
            right_scheduler_socket,
            right_link,
            right_scheduler_reserved,
            right_scheduler_stats,
            epoch,
        )
    });

    let metrics_left_stats = Arc::clone(&left_to_right_stats);
    let metrics_right_stats = Arc::clone(&right_to_left_stats);
    let metrics_left_reserved = Arc::clone(&left_to_right_reserved);
    let metrics_right_reserved = Arc::clone(&right_to_left_reserved);
    let metrics_left_socket = Arc::clone(&left_socket);
    let metrics_right_socket = Arc::clone(&right_socket);
    let metrics_interval = config.metrics_interval;
    thread::spawn(move || loop {
        thread::sleep(metrics_interval);
        if let Ok(drops) = metrics_left_socket.take_kernel_drops() {
            metrics_left_stats
                .kernel_socket_drops
                .fetch_add(drops, Ordering::Relaxed);
        }
        if let Ok(drops) = metrics_right_socket.take_kernel_drops() {
            metrics_right_stats
                .kernel_socket_drops
                .fetch_add(drops, Ordering::Relaxed);
        }
        println!(
            "{{\"left_to_right\":{},\"right_to_left\":{}}}",
            metrics_left_stats.snapshot(metrics_left_reserved.load(Ordering::Relaxed)),
            metrics_right_stats.snapshot(metrics_right_reserved.load(Ordering::Relaxed)),
        );
    });

    let left_receive_socket = Arc::clone(&left_socket);
    let left_path = ReceivePath {
        destination_subnet: config.right_subnet,
        egress_mac: right_socket.mac_address,
        next_hop_mac: config.right_next_hop_mac,
        link: config.left_to_right,
        seed: config.seed,
        queue: left_to_right_sender,
        reserved_queue_bytes: left_to_right_reserved,
        stats: left_to_right_stats,
    };
    let left_receiver = thread::spawn(move || receive_loop(left_receive_socket, left_path));

    let right_receive_socket = Arc::clone(&right_socket);
    let right_path = ReceivePath {
        destination_subnet: config.left_subnet,
        egress_mac: left_socket.mac_address,
        next_hop_mac: config.left_next_hop_mac,
        link: config.right_to_left,
        seed: config.seed ^ 0xa5a5_5a5a_9e37_79b9,
        queue: right_to_left_sender,
        reserved_queue_bytes: right_to_left_reserved,
        stats: right_to_left_stats,
    };
    let right_receiver = thread::spawn(move || receive_loop(right_receive_socket, right_path));

    join_receiver("left", left_receiver)?;
    join_receiver("right", right_receiver)?;
    Ok(())
}

fn receive_loop(socket: Arc<RawPacketSocket>, path: ReceivePath) -> io::Result<()> {
    let mut buffer = vec![0_u8; MAX_FRAME_LEN];
    let mut random = XorShift64::new(path.seed);
    loop {
        let (frame_len, packet_type) = match socket.receive(&mut buffer) {
            Ok(received) => received,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if packet_type == PACKET_OUTGOING || packet_type == PACKET_LOOPBACK {
            continue;
        }

        path.stats.received_packets.fetch_add(1, Ordering::Relaxed);
        path.stats
            .received_bytes
            .fetch_add(frame_len as u64, Ordering::Relaxed);
        let mut frame = buffer[..frame_len].to_vec();
        if let Err(reason) = prepare_ipv4_frame(
            &mut frame,
            path.destination_subnet,
            path.egress_mac,
            path.next_hop_mac,
        ) {
            path.stats.record_prepare_drop(reason);
            continue;
        }
        if random.should_drop(path.link.loss_fraction) {
            path.stats.random_drops.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let frame_bytes = frame.len() as u64;
        if !try_reserve_queue_bytes(
            &path.reserved_queue_bytes,
            frame_bytes,
            path.link.queue_bytes,
            &path.stats.peak_queue_bytes,
        ) {
            path.stats.queue_drops.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let queued = QueuedFrame {
            bytes: frame,
            arrival: Instant::now(),
        };
        if let Err(error) = path.queue.send(queued) {
            let bytes = error.0.bytes.len() as u64;
            path.reserved_queue_bytes
                .fetch_sub(bytes, Ordering::Relaxed);
            path.stats.channel_drops.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn scheduler_loop(
    receiver: Receiver<QueuedFrame>,
    egress_socket: Arc<RawPacketSocket>,
    link: LinkConfig,
    reserved_queue_bytes: Arc<AtomicU64>,
    stats: Arc<DirectionStats>,
    epoch: Instant,
) {
    let mut queue = VecDeque::<ScheduledFrame>::new();
    let mut clock = LinkClock::default();
    let mut disconnected = false;

    loop {
        let now = Instant::now();
        release_serialized_queue_bytes(&mut queue, &reserved_queue_bytes, now);
        while queue
            .front()
            .is_some_and(|scheduled| scheduled.delivery <= now)
        {
            let scheduled = queue.pop_front().expect("front existed");
            let frame_len = scheduled.bytes.len() as u64;
            if scheduled.queue_accounted {
                reserved_queue_bytes.fetch_sub(frame_len, Ordering::Relaxed);
            }
            match egress_socket.send(&scheduled.bytes) {
                Ok(()) => {
                    stats.forwarded_packets.fetch_add(1, Ordering::Relaxed);
                    stats
                        .forwarded_bytes
                        .fetch_add(frame_len, Ordering::Relaxed);
                }
                Err(error) => {
                    stats.send_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "lab-router: send on {} failed: {error}",
                        egress_socket.interface_name
                    );
                }
            }
        }

        if disconnected && queue.is_empty() {
            return;
        }

        let next_serialization = queue
            .iter()
            .find(|scheduled| scheduled.queue_accounted)
            .map(|scheduled| scheduled.serialization_complete);
        let next_delivery = queue.front().map(|scheduled| scheduled.delivery);
        let timeout = next_serialization
            .into_iter()
            .chain(next_delivery)
            .min()
            .map(|event| event.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_millis(100));
        if disconnected {
            thread::sleep(timeout.min(Duration::from_millis(100)));
            continue;
        }

        match receiver.recv_timeout(timeout) {
            Ok(frame) => {
                let arrival = frame.arrival.saturating_duration_since(epoch);
                let delivery =
                    clock.schedule(arrival, frame.bytes.len(), link.rate_bps, link.delay);
                queue.push_back(ScheduledFrame {
                    bytes: frame.bytes,
                    serialization_complete: epoch + delivery - link.delay,
                    delivery: epoch + delivery,
                    queue_accounted: true,
                });
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => disconnected = true,
        }
    }
}

fn release_serialized_queue_bytes(
    queue: &mut VecDeque<ScheduledFrame>,
    reserved_queue_bytes: &AtomicU64,
    now: Instant,
) {
    for scheduled in queue.iter_mut() {
        if !scheduled.queue_accounted {
            continue;
        }
        if scheduled.serialization_complete > now {
            break;
        }
        reserved_queue_bytes.fetch_sub(scheduled.bytes.len() as u64, Ordering::Relaxed);
        scheduled.queue_accounted = false;
    }
}

fn try_reserve_queue_bytes(
    reserved: &AtomicU64,
    frame_bytes: u64,
    queue_limit: u64,
    peak: &AtomicU64,
) -> bool {
    let mut current = reserved.load(Ordering::Relaxed);
    loop {
        let Some(next) = current.checked_add(frame_bytes) else {
            return false;
        };
        if next > queue_limit {
            return false;
        }
        match reserved.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => {
                peak.fetch_max(next, Ordering::Relaxed);
                return true;
            }
            Err(actual) => current = actual,
        }
    }
}

fn join_receiver(name: &str, handle: thread::JoinHandle<io::Result<()>>) -> Result<(), String> {
    match handle.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("{name} receive loop failed: {error}")),
        Err(_) => Err(format!("{name} receive loop panicked")),
    }
}

fn read_interface_index(interface_name: &str) -> io::Result<i32> {
    fs::read_to_string(format!("/sys/class/net/{interface_name}/ifindex"))?
        .trim()
        .parse::<i32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid interface index"))
}

fn read_interface_mac(interface_name: &str) -> io::Result<MacAddress> {
    fs::read_to_string(format!("/sys/class/net/{interface_name}/address"))?
        .trim()
        .parse::<MacAddress>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn parse_config() -> Result<Config, String> {
    let options = parse_options()?;
    reject_unknown_options(&options)?;
    let left_to_right = parse_link(&options, "left-to-right")?;
    let right_to_left = parse_link(&options, "right-to-left")?;
    Ok(Config {
        left_interface: required(&options, "left-if")?.to_string(),
        right_interface: required(&options, "right-if")?.to_string(),
        left_subnet: parse_required(&options, "left-subnet")?,
        right_subnet: parse_required(&options, "right-subnet")?,
        left_next_hop_mac: parse_required(&options, "left-next-hop-mac")?,
        right_next_hop_mac: parse_required(&options, "right-next-hop-mac")?,
        left_to_right,
        right_to_left,
        seed: parse_optional(&options, "seed", 1_u64)?,
        metrics_interval: Duration::from_millis(parse_optional(
            &options,
            "metrics-interval-ms",
            1000_u64,
        )?),
    })
}

fn parse_link(options: &HashMap<String, String>, prefix: &str) -> Result<LinkConfig, String> {
    let delay_ms = parse_optional(options, &format!("{prefix}-delay-ms"), 14.0_f64)?;
    let rate_mbps = parse_optional(options, &format!("{prefix}-rate-mbps"), 1000.0_f64)?;
    let loss_percent = parse_optional(options, &format!("{prefix}-loss-percent"), 0.0_f64)?;
    let queue_bytes = parse_optional(options, &format!("{prefix}-queue-bytes"), 4_194_304_u64)?;
    if !delay_ms.is_finite() || delay_ms < 0.0 {
        return Err(format!("{prefix} delay must be finite and non-negative"));
    }
    if !rate_mbps.is_finite() || rate_mbps <= 0.0 {
        return Err(format!("{prefix} rate must be finite and positive"));
    }
    if !loss_percent.is_finite() || !(0.0..=100.0).contains(&loss_percent) {
        return Err(format!("{prefix} loss must be between 0 and 100 percent"));
    }
    if queue_bytes < 64 {
        return Err(format!("{prefix} queue must be at least 64 bytes"));
    }
    Ok(LinkConfig {
        delay: Duration::from_secs_f64(delay_ms / 1000.0),
        rate_bps: rate_mbps * 1e6,
        loss_fraction: loss_percent / 100.0,
        queue_bytes,
    })
}

fn parse_options() -> Result<HashMap<String, String>, String> {
    let mut options = HashMap::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let Some(name) = argument.strip_prefix("--") else {
            return Err(format!("unexpected positional argument: {argument}"));
        };
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for --{name}"))?;
        options.insert(name.to_string(), value);
    }
    Ok(options)
}

fn reject_unknown_options(options: &HashMap<String, String>) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "left-if",
        "right-if",
        "left-subnet",
        "right-subnet",
        "left-next-hop-mac",
        "right-next-hop-mac",
        "left-to-right-delay-ms",
        "right-to-left-delay-ms",
        "left-to-right-rate-mbps",
        "right-to-left-rate-mbps",
        "left-to-right-loss-percent",
        "right-to-left-loss-percent",
        "left-to-right-queue-bytes",
        "right-to-left-queue-bytes",
        "seed",
        "metrics-interval-ms",
    ];
    for option in options.keys() {
        if !ALLOWED.contains(&option.as_str()) {
            return Err(format!("unknown option: --{option}"));
        }
    }
    Ok(())
}

fn required<'a>(options: &'a HashMap<String, String>, name: &str) -> Result<&'a str, String> {
    options
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option --{name}"))
}

fn parse_required<T>(options: &HashMap<String, String>, name: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    required(options, name)?
        .parse::<T>()
        .map_err(|error| format!("invalid --{name}: {error}"))
}

fn parse_optional<T>(options: &HashMap<String, String>, name: &str, default: T) -> Result<T, String>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    options.get(name).map_or(Ok(default), |value| {
        value
            .parse::<T>()
            .map_err(|error| format!("invalid --{name}: {error}"))
    })
}

fn print_help() {
    println!(
        r#"Rust IPv4 laboratory router

Required:
  --left-if IFACE
  --right-if IFACE
  --left-subnet CIDR
  --right-subnet CIDR
  --left-next-hop-mac MAC
  --right-next-hop-mac MAC

Per-direction impairment options:
  --left-to-right-delay-ms N       default: 14
  --right-to-left-delay-ms N      default: 14
  --left-to-right-rate-mbps N     default: 1000
  --right-to-left-rate-mbps N    default: 1000
  --left-to-right-loss-percent N  default: 0
  --right-to-left-loss-percent N default: 0
  --left-to-right-queue-bytes N   default: 4194304
  --right-to-left-queue-bytes N  default: 4194304

Other:
  --seed N                        default: 1
  --metrics-interval-ms N         default: 1000
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_reservation_enforces_drop_tail_limit() {
        let reserved = AtomicU64::new(0);
        let peak = AtomicU64::new(0);
        assert!(try_reserve_queue_bytes(&reserved, 700, 1000, &peak));
        assert!(!try_reserve_queue_bytes(&reserved, 400, 1000, &peak));
        assert_eq!(reserved.load(Ordering::Relaxed), 700);
        assert_eq!(peak.load(Ordering::Relaxed), 700);
    }
}
