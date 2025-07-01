# Basic Concepts

This chapter introduces the fundamental concepts that make Zenoh unique. Understanding these concepts will help you design effective applications and make the most of Zenoh's capabilities.

## The Zenoh Data Model

### Key Expressions

At the heart of Zenoh is the concept of **key expressions** - hierarchical identifiers that organize data in a tree-like structure, similar to file paths or URLs.

```
robot/sensor/temperature
robot/sensor/pressure  
robot/actuator/motor/speed
building/floor1/room101/temperature
building/floor1/room101/humidity
```

Key expressions use forward slashes (`/`) as separators and create natural hierarchies that make data organization intuitive.

#### Wildcards and Pattern Matching

Zenoh supports powerful pattern matching with wildcards:

- **Single-level wildcard (`*`)** - Matches exactly one level
  ```
  robot/sensor/*           # Matches: robot/sensor/temperature, robot/sensor/pressure
                           # Does NOT match: robot/sensor/temp/reading
  ```

- **Multi-level wildcard (`**`)** - Matches any number of levels
  ```
  robot/**                 # Matches: robot/sensor/temperature, robot/actuator/motor/speed
  building/floor1/**       # Matches: building/floor1/room101/temperature, building/floor1/lobby/light
  ```

#### Best Practices for Key Expressions

1. **Use hierarchical organization**: Group related data under common prefixes
2. **Be specific but flexible**: Balance precision with reusability
3. **Use consistent naming**: Stick to conventions (snake_case, kebab-case, etc.)
4. **Consider your access patterns**: Design hierarchies that match how you'll query data

Examples of good key expressions:
```
vehicle/engine/temperature
vehicle/gps/location
sensor/environment/temperature
sensor/environment/humidity
config/database/connection_string
```

### Values and Encodings

Every piece of data in Zenoh consists of:
- **Payload**: The actual data bytes
- **Encoding**: Metadata describing how to interpret the payload

Common encodings include:
- `text/plain` - UTF-8 text
- `application/json` - JSON data
- `application/octet-stream` - Raw binary data
- `application/integer` - Integer values
- `application/float` - Floating-point values

```rust
// Different ways to create values
let text_value = Value::from("Hello, World!");
let json_value = Value::from(r#"{"temperature": 23.5}"#)
    .with_encoding(Encoding::APPLICATION_JSON);
let binary_value = Value::from(vec![0x01, 0x02, 0x03, 0x04]);
```

## Zenoh Operations

### The Three Core Operations

Zenoh provides three fundamental operations that handle all data interaction:

#### 1. Put - Writing Data
The `put` operation writes data to a key expression:

```rust
// Write a temperature reading
session.put("robot/sensor/temperature", "23.5").res().await?;

// Write JSON data
session.put("robot/status", r#"{"online": true, "battery": 85}"#)
    .encoding(Encoding::APPLICATION_JSON)
    .res().await?;
```

#### 2. Get - Querying Data
The `get` operation queries data from the network:

```rust
// Query current temperature
let replies = session.get("robot/sensor/temperature").res().await?;

// Query all sensor data
let replies = session.get("robot/sensor/*").res().await?;
```

#### 3. Subscribe - Continuous Updates
The `subscribe` operation receives continuous updates:

```rust
let subscriber = session
    .declare_subscriber("robot/sensor/temperature")
    .callback(|sample| {
        println!("Temperature: {}", sample.value());
    })
    .res().await?;
```

### Publishers and Queryables

While put/get/subscribe are the basic operations, Zenoh also provides optimized declarations:

#### Publishers
Publishers optimize repeated writes to the same key expression:

```rust
let publisher = session.declare_publisher("robot/sensor/temperature").res().await?;

// Publishing is now more efficient
for reading in sensor_readings {
    publisher.put(reading.to_string()).res().await?;
}
```

#### Queryables
Queryables respond to queries with computed or stored data:

```rust
let queryable = session
    .declare_queryable("robot/system/info")
    .callback(|query| {
        let info = get_system_info(); // Your function
        query.reply(Ok(Sample::new("robot/system/info", info))).res_async();
    })
    .res().await?;
```

## Selectors and Advanced Querying

**Selectors** extend key expressions with additional query parameters and filters.

### Basic Selector Syntax

```
key_expression?(parameters)
```

Examples:
```
robot/sensor/temperature                    # Simple key expression
robot/sensor/*                             # With wildcards
robot/sensor/*?(encoding=application/json) # With parameter filter
**?(timestamp>2024-01-01T00:00:00Z)        # With time filter
```

### Query Parameters

Selectors can include parameters that modify query behavior:

```rust
// Query with time range
session.get("sensor/temperature?(start_time=2024-01-01T00:00:00Z&end_time=2024-01-02T00:00:00Z)")

// Query with consolidation mode
session.get("sensor/*?(consolidation=latest)")

// Query with custom parameters
session.get("api/data?(limit=100&offset=50)")
```

## Zenoh Nodes and Modes

Zenoh nodes can operate in three different modes, each with distinct characteristics:

### Client Mode
- **Lightweight**: Minimal resource usage
- **Dependent**: Requires connection to router or peer
- **Simple**: Easiest to configure and deploy
- **Use case**: IoT devices, simple applications

```rust
let config = config::client([("tcp/192.168.1.1:7447").parse()?]);
let session = zenoh::open(config).res().await?;
```

### Peer Mode  
- **Autonomous**: Can operate without infrastructure
- **Collaborative**: Participates in routing decisions
- **Resilient**: Can form mesh networks
- **Use case**: Edge computing, distributed systems

```rust
let config = config::peer();
let session = zenoh::open(config).res().await?;
```

### Router Mode
- **Infrastructure**: Provides routing services to others
- **Scalable**: Handles many client connections
- **Optimized**: Advanced routing algorithms
- **Use case**: Data centers, network infrastructure

```bash
# Typically run as a separate process
zenohd --config router.json5
```

## Sessions and Lifecycle

A **session** represents a connection to the Zenoh network and is the entry point for all operations.

### Session Creation
```rust
// Default configuration (auto-detects best mode)
let session = zenoh::open(config::default()).res().await?;

// Explicit configuration
let mut config = config::peer();
config.timestamping.set_enabled(Some(true))?;
let session = zenoh::open(config).res().await?;
```

### Session Management
- Sessions are **automatically managed** - no manual connection handling
- **Reconnection** happens transparently if network issues occur
- **Resource cleanup** is automatic when the session is dropped

### Multiple Sessions
You can create multiple sessions for different purposes:

```rust
// High-priority control session
let control_session = zenoh::open(config::peer()).res().await?;

// Data streaming session
let data_session = zenoh::open(config::client(locators)).res().await?;
```

## Data Flow Patterns

### Publish/Subscribe Pattern
Perfect for real-time data streaming:

```
[Sensor] --publish--> [Zenoh Network] --deliver--> [Dashboard]
                                    |
                                    +--deliver--> [Database]
                                    |
                                    +--deliver--> [Alert System]
```

### Query/Reply Pattern
Ideal for request/response interactions:

```
[Client] --query--> [Zenoh Network] --forward--> [Queryable]
                                              |
         <--reply-- [Zenoh Network] <--reply--+
```

### Unified Pattern
Zenoh's power comes from unifying these patterns:

```rust
// A queryable can respond with live data, stored data, or computed results
let queryable = session
    .declare_queryable("sensor/temperature")
    .callback(|query| {
        let response = match query.parameters() {
            Some(params) if params.contains("live") => get_live_temperature(),
            Some(params) if params.contains("average") => get_average_temperature(),
            _ => get_current_temperature(),
        };
        query.reply(Ok(Sample::new("sensor/temperature", response))).res_async();
    })
    .res().await?;
```

## Quality of Service (QoS)

Zenoh provides several QoS options to control data delivery:

### Reliability
- **Best Effort**: Fast, may lose data under network stress
- **Reliable**: Guaranteed delivery, slower

```rust
// Best effort (default for pub/sub)
publisher.put("data").res().await?;

// Reliable
publisher.put("important_data")
    .reliability(Reliability::Reliable)
    .res().await?;
```

### Congestion Control
Controls behavior under network congestion:

```rust
publisher.put("data")
    .congestion_control(CongestionControl::Block)  // Wait for network capacity
    .res().await?;

publisher.put("data")
    .congestion_control(CongestionControl::Drop)   // Drop if congested
    .res().await?;
```

### Priority
Control message priority (0-7, where 7 is highest):

```rust
publisher.put("urgent_data")
    .priority(Priority::RealTime)  // Highest priority
    .res().await?;
```

## Distributed Architecture

### Automatic Discovery
Zenoh nodes automatically discover each other using:
- **Multicast scouting** on local networks
- **Gossip protocol** for peer-to-peer discovery
- **Manual configuration** for specific deployments

### Routing and Forwarding
- **Intelligent routing** based on subscriptions and queryables
- **Loop prevention** through sophisticated algorithms
- **Load balancing** across multiple paths

### Fault Tolerance
- **Automatic reconnection** when links fail
- **Route adaptation** to network changes  
- **Graceful degradation** under stress

## Understanding Data Flow

When you publish data in Zenoh:

1. **Local Processing**: The publisher processes your data
2. **Routing Decision**: Zenoh determines which nodes need the data
3. **Network Transport**: Data flows over the most efficient paths
4. **Remote Delivery**: Subscribers receive the data
5. **Callback Execution**: Your callback functions process received data

When you query data:

1. **Query Propagation**: Your query spreads through the network
2. **Matching**: Queryables with matching key expressions respond
3. **Response Collection**: Replies are gathered and optionally consolidated
4. **Result Delivery**: You receive the consolidated results

## Next Steps

Now that you understand Zenoh's basic concepts, you're ready to:

1. **Explore specific node types** in the [Core Concepts](../concepts/) section
2. **Learn detailed programming techniques** in the [Programming Guide](../programming/)
3. **Understand advanced routing** in [Advanced Topics](../advanced/)
4. **Add persistence** with [Storage](../storage/)

These concepts form the foundation for everything in Zenoh. As you work with more advanced features, you'll see how they all build upon these core ideas to create a powerful, unified data platform.