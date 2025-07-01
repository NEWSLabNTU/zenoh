# What is Zenoh?

Zenoh is a revolutionary data communication protocol that fundamentally changes how we think about distributed data management. Unlike traditional approaches that separate data movement, storage, and computation into distinct layers, Zenoh unifies these concepts into a single, coherent framework.

## The Three Pillars of Data

Zenoh addresses what we call the "three pillars of data" in distributed systems:

1. **Data in Motion** - Real-time streaming data between applications
2. **Data at Rest** - Persistent storage and historical data access
3. **Computations** - Processing and transformation of data

Most existing systems treat these as separate concerns, requiring complex integration between different technologies. Zenoh eliminates this complexity by providing a unified API that seamlessly handles all three scenarios.

## Core Philosophy

The fundamental philosophy behind Zenoh is **seamless data flow**. Whether you're publishing real-time sensor data, querying historical information, or triggering computations, you use the same simple primitives:

- **Put** - Store or update data
- **Get** - Retrieve data (real-time or historical)
- **Subscribe** - Receive updates when data changes

This simplicity masks sophisticated underlying mechanisms that automatically handle routing, storage, caching, and distribution across your network.

## Key Differentiators

### Location-Independent Data Access
With Zenoh, applications don't need to know where data comes from or how it's stored. A query for sensor data might be answered by:
- A live sensor publishing real-time values
- Historical data from a storage backend
- A computed result from an edge processing node

### Zero-Copy Architecture
Zenoh is built from the ground up for performance. Its zero-copy design minimizes memory allocations and data copying, enabling high-throughput, low-latency communication even in resource-constrained environments.

### Protocol Agnostic
Zenoh can run over any transport protocol - TCP, UDP, QUIC, WebSocket, or even shared memory. This flexibility allows it to adapt to different network conditions and requirements.

### Hierarchical Addressing
Zenoh uses a hierarchical key expression system similar to file paths or URLs. This enables powerful pattern matching and allows applications to subscribe to entire data hierarchies with simple wildcard expressions.

## Use Cases and Applications

### Internet of Things (IoT)
Zenoh excels in IoT scenarios where you need to:
- Collect data from thousands of sensors
- Store historical trends for analysis
- Implement edge processing for real-time decisions
- Handle intermittent connectivity

### Robotics
In robotics applications, Zenoh provides:
- Low-latency sensor data distribution
- Coordinated control across multiple robots
- Seamless integration with external systems
- Efficient bandwidth usage for remote operations

### Edge Computing
For edge computing deployments:
- Automatic data synchronization between edge and cloud
- Local processing with global visibility
- Adaptive behavior based on network conditions
- Simplified application development

### Automotive
In automotive systems:
- Vehicle-to-vehicle communication
- Integration of multiple ECUs and sensors
- Over-the-air updates and diagnostics
- Fleet management and analytics

## Performance Characteristics

Zenoh is designed for extreme performance:

- **Throughput**: Handles millions of messages per second
- **Latency**: Sub-millisecond latency for local communication
- **Scalability**: Supports thousands of nodes in a single network
- **Efficiency**: Minimal CPU and memory overhead

## How Zenoh Differs from Other Protocols

### vs. MQTT
While MQTT is excellent for simple IoT scenarios, Zenoh offers:
- Native support for queries and storage
- Better performance and lower overhead
- More sophisticated routing capabilities
- Built-in redundancy and failover

### vs. DDS
Compared to DDS, Zenoh provides:
- Simpler programming model
- Better performance in many scenarios
- Unified approach to data and storage
- More flexible deployment options

### vs. Apache Kafka
Unlike Kafka, Zenoh offers:
- Real-time data access without logging overhead
- Direct peer-to-peer communication
- Built-in storage capabilities
- Lower operational complexity

## The Zenoh Ecosystem

Zenoh is more than just a protocol - it's a complete ecosystem:

### Core Runtime
The Zenoh runtime provides the fundamental communication and routing capabilities. It can operate in three modes:
- **Client** - Lightweight applications that connect to the network
- **Peer** - Nodes that participate in routing decisions
- **Router** - Infrastructure nodes that provide full routing services

### Language Bindings
Zenoh provides native APIs for multiple programming languages:
- Rust (reference implementation)
- Python
- C/C++
- Java
- JavaScript

### Plugins and Extensions
The plugin system allows extending Zenoh with:
- Storage backends (databases, filesystems)
- Protocol bridges (MQTT, DDS, HTTP)
- Custom processing logic
- Monitoring and observability tools

### Tools and Utilities
The ecosystem includes command-line tools for:
- Network exploration and debugging
- Performance testing and benchmarking
- Configuration management
- Data visualization

## Getting Started

Ready to explore Zenoh? The next sections will guide you through:
- Installing Zenoh on your system
- Running your first Zenoh application
- Understanding the core concepts in detail

Zenoh's design philosophy of "simple on the surface, powerful underneath" means you can start with basic examples and gradually explore more advanced features as your needs evolve.