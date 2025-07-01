# Zenoh Architecture Documentation

This directory contains detailed documentation about Zenoh's system architecture.

## Overview

Zenoh is a pub/sub/query protocol that unifies data in motion, data at rest, and computations. It provides a unified API for accessing data across distributed systems while maintaining high performance and scalability.

## Architecture Documents

1. **[Core Components](core-components.md)** - Detailed breakdown of Zenoh's core components
2. **[Networking Stack](networking-stack.md)** - Transport and link layer design
3. **[Routing Protocol](routing-protocol.md)** - How Zenoh routes data between nodes
4. **[Storage System](storage-system.md)** - Persistent storage architecture
5. **[Plugin System](plugin-system.md)** - Extensibility through plugins

## Key Design Principles

1. **Zero-Copy Architecture** - Minimize data copies for performance
2. **Protocol Agnostic** - Support multiple transport protocols
3. **Hierarchical Addressing** - Key expressions with wildcards
4. **Pluggable Routing** - Different routing strategies for different scenarios
5. **Extensibility** - Plugin system for custom functionality

## System Overview Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Applications                             │
├─────────────────────────────────────────────────────────────┤
│                    Zenoh API (Session)                       │
├─────────────────────────────────────────────────────────────┤
│                      Runtime                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Router    │  │   Plugins   │  │    HLC      │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
├─────────────────────────────────────────────────────────────┤
│                   Routing Layer (HAT)                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │
│  │ Client  │  │  P2P    │  │Linkstate│  │ Router  │      │
│  │  HAT    │  │  HAT    │  │  HAT    │  │  HAT    │      │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘      │
├─────────────────────────────────────────────────────────────┤
│                   Transport Layer                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │
│  │   TCP   │  │   UDP   │  │  QUIC   │  │   TLS   │      │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Node Types

Zenoh supports three types of nodes:

1. **Client** - Lightweight nodes that connect to routers/peers
2. **Peer** - Nodes that can route data and connect to other peers
3. **Router** - Infrastructure nodes with full routing capabilities

## Data Flow Patterns

1. **Pub/Sub** - Publishers send data to subscribers
2. **Query/Reply** - Clients query data from queryables
3. **Liveliness** - Monitor node presence
4. **Storage** - Persist data with time-series support