# Zenoh Networking Stack

This document describes Zenoh's networking and transport layer architecture.

## Overview

Zenoh's networking stack is designed to be protocol-agnostic and highly performant, supporting multiple transport protocols while maintaining a unified abstraction layer.

## Architecture Layers

```
┌─────────────────────────────────────────────────┐
│            Zenoh API Layer                      │
├─────────────────────────────────────────────────┤
│            Transport Manager                     │
├─────────────────────────────────────────────────┤
│      Transport Abstraction (Unicast/Multicast)  │
├─────────────────────────────────────────────────┤
│            Link Manager                         │
├─────────────────────────────────────────────────┤
│      Link Implementations                       │
│  ┌─────┐ ┌─────┐ ┌──────┐ ┌─────┐ ┌────────┐ │
│  │ TCP │ │ UDP │ │ QUIC │ │ TLS │ │Websocket│ │
│  └─────┘ └─────┘ └──────┘ └─────┘ └────────┘ │
└─────────────────────────────────────────────────┘
```

## Core Components

### 1. Transport Manager (io/zenoh-transport/src/manager.rs)

Manages the lifecycle of transport connections.

**Responsibilities:**
- Connection establishment and teardown
- Transport protocol selection
- Connection multiplexing
- Keep-alive management

**Key Configuration:**
```rust
pub struct TransportManagerConfig {
    pub unicast: TransportUnicastConfig,
    pub multicast: TransportMulticastConfig,
    pub endpoints: Vec<EndPoint>,
    pub ttl: Option<u8>,
}
```

### 2. Transport Abstraction

#### Unicast Transport
Point-to-point reliable communication:
```rust
pub struct TransportUnicast {
    config: TransportUnicastConfig,
    manager: TransportManager,
    procs: Arc<TransportUnicastProcs>,
    token: CancellationToken,
}
```

**Features:**
- Reliable, ordered delivery
- Fragmentation/reassembly
- Flow control
- Multiple links per transport

#### Multicast Transport
One-to-many communication:
```rust
pub struct TransportMulticast {
    config: TransportMulticastConfig,
    manager: TransportManager,
    procs: Arc<TransportMulticastProcs>,
    token: CancellationToken,
}
```

**Features:**
- Best-effort delivery
- Group management
- Efficient data distribution

### 3. Link Layer

Abstract interface for different protocols:

```rust
pub trait Link: Send + Sync {
    fn get_locator(&self) -> &Locator;
    fn get_mtu(&self) -> u16;
    fn is_reliable(&self) -> bool;
    fn is_encrypted(&self) -> bool;
    async fn send(&self, buffer: &[u8]) -> Result<()>;
    async fn recv(&self) -> Result<Bytes>;
}
```

## Protocol Implementations

### TCP (io/zenoh-links/zenoh-link-tcp/)
- Reliable, stream-oriented
- Supports TCP_NODELAY for low latency
- Configurable send/receive buffers

### UDP (io/zenoh-links/zenoh-link-udp/)
- Unreliable, datagram-oriented
- Used for multicast scenarios
- Lower overhead than TCP

### QUIC (io/zenoh-links/zenoh-link-quic/)
- Multiplexed streams over UDP
- Built-in encryption
- Better performance over lossy networks

### TLS (io/zenoh-links/zenoh-link-tls/)
- Encrypted TCP connections
- Certificate-based authentication
- Configurable cipher suites

### WebSocket (io/zenoh-links/zenoh-link-ws/)
- Web-compatible transport
- Works through proxies/firewalls
- Both WS and WSS supported

## Protocol Features

### 1. Fragmentation and Reassembly

Large messages are fragmented at the transport layer:

```rust
pub struct Fragmentation {
    sn: SequenceNumber,
    mtu: u16,
    fragbuf: HashMap<SequenceNumber, FragmentBuffer>,
}

// Fragment large messages
fn fragment_message(msg: &Message, mtu: u16) -> Vec<Fragment> {
    let chunks = msg.payload.chunks(mtu - HEADER_SIZE);
    chunks.enumerate().map(|(i, chunk)| {
        Fragment {
            sn: msg.sn,
            index: i as u16,
            total: chunks.len() as u16,
            data: chunk.to_vec(),
        }
    }).collect()
}
```

### 2. Reliability and Ordering

Configurable per transport:
- **Reliable**: All messages delivered in order
- **Best Effort**: Messages may be lost/reordered

```rust
pub enum Reliability {
    Reliable,
    BestEffort,
}

pub struct ReliabilityQueue {
    sn: SequenceNumber,
    pending: BTreeMap<SequenceNumber, Message>,
    window: u32,
}
```

### 3. Keep-Alive Mechanism

Detects failed connections:
```rust
pub struct KeepAlive {
    interval: Duration,
    timeout: Duration,
    last_recv: Instant,
    timer: Timer,
}

// Periodic keep-alive
async fn keep_alive_loop(&self) {
    loop {
        sleep(self.interval).await;
        if self.last_recv.elapsed() > self.timeout {
            self.on_timeout();
            break;
        }
        self.send_keep_alive().await;
    }
}
```

### 4. Flow Control

Prevents overwhelming receivers:
- Window-based flow control
- Backpressure propagation
- Adaptive rate limiting

## Message Format

### Wire Protocol

```
┌─────────────┬─────────────┬─────────────┬─────────────┐
│   Header    │   Body      │  Extension  │   Payload   │
│   (8 bits)  │  (variable) │  (optional) │  (variable) │
└─────────────┴─────────────┴─────────────┴─────────────┘
```

### Message Types
- **Scout**: Node discovery
- **Hello**: Connection establishment
- **Join**: Session establishment
- **Init/Open/Close**: Session management
- **Frame**: Data transfer
- **Fragment**: Large message parts
- **KeepAlive**: Connection liveness

## Performance Optimizations

### 1. Zero-Copy Design

Minimize data copies through the stack:
```rust
pub struct ZBuf {
    slices: Vec<ZSlice>,
}

pub struct ZSlice {
    data: Arc<dyn Buffer>,
    start: usize,
    len: usize,
}
```

### 2. Batch Processing

Aggregate multiple messages:
```rust
pub struct Batcher {
    buffer: Vec<u8>,
    mtu: usize,
    flush_timer: Timer,
}

fn add_message(&mut self, msg: Message) -> Option<Vec<u8>> {
    if self.buffer.len() + msg.len() > self.mtu {
        return Some(self.flush());
    }
    self.buffer.extend_from_slice(&msg.encode());
    None
}
```

### 3. Connection Multiplexing

Multiple logical channels over single connection:
- Reduces connection overhead
- Better resource utilization
- Improved latency for small messages

### 4. Shared Memory Support

Optional zero-copy for local communication:
```rust
pub struct ShmTransport {
    segments: HashMap<SegmentId, SharedMemory>,
    allocator: ShmAllocator,
}
```

## Configuration

### Transport Configuration

```json
{
  "transport": {
    "unicast": {
      "accept_timeout": 10000,
      "max_sessions": 1000,
      "max_links": 1,
      "lowlatency": false
    },
    "multicast": {
      "enabled": true,
      "group": "224.0.0.1:7447",
      "interface": "auto"
    },
    "link": {
      "protocols": ["tcp", "udp", "quic"],
      "tls": {
        "root_ca": "/path/to/ca.pem",
        "client_cert": "/path/to/cert.pem"
      }
    }
  }
}
```

## Error Handling

Transport errors are categorized:
1. **Transient**: Temporary failures (retry)
2. **Permanent**: Unrecoverable errors (close)
3. **Configuration**: Invalid settings

```rust
pub enum TransportError {
    Transient(String),
    Permanent(String),
    Configuration(String),
}
```

## Security Considerations

1. **Authentication**: TLS certificates, pre-shared keys
2. **Encryption**: TLS, QUIC built-in encryption
3. **Access Control**: Link-level authorization
4. **DoS Protection**: Rate limiting, connection limits