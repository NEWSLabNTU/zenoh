# Zenoh Routing Protocol

This document describes Zenoh's routing protocol implementation and algorithms.

## Overview

Zenoh uses a pluggable routing architecture that supports different routing strategies based on node types and deployment scenarios. The routing system is built around the concept of "faces" (connections) and hierarchical resources.

## Routing Strategies

### 1. Client Routing (Client HAT)

Clients use the simplest routing strategy:
- No direct routing between clients
- All routing decisions delegated to connected routers/peers
- Minimal state maintenance

### 2. P2P Peer Routing

Basic peer-to-peer routing without full topology knowledge:
- Uses gossip protocol for peer discovery
- Direct routing to known peers
- Falls back to connected routers for unknown destinations

### 3. Link-State Peer Routing

Full link-state routing protocol implementation:

**Algorithm:**
```rust
// Simplified link-state algorithm
fn compute_routes(&mut self) {
    // 1. Build network graph from link-state updates
    let graph = build_topology_graph();
    
    // 2. Compute shortest paths using Bellman-Ford
    for node in graph.nodes() {
        let paths = bellman_ford(&graph, node);
        self.routing_table.insert(node, paths);
    }
    
    // 3. Build spanning trees for efficient multicast
    self.trees = compute_spanning_trees(&graph);
}
```

**Key Features:**
- Complete network topology knowledge
- Weighted links for QoS-aware routing
- Automatic rerouting on topology changes
- Efficient multicast trees

### 4. Router Routing (Router HAT)

Infrastructure nodes with full routing capabilities:
- Maintains global routing state
- Handles peer failover brokering
- Supports complex routing policies

## Routing Tables

### Resource Tree Structure

```
Tables
├── Resources (Hierarchical Tree)
│   ├── /robot
│   │   ├── /sensor
│   │   │   ├── /temp → [Face1, Face3]
│   │   │   └── /pressure → [Face2]
│   │   └── /cmd → [Face1, Face4]
│   └── /monitoring → [Face5]
├── Faces (Connection Map)
│   ├── Face1 → {type: Peer, zid: 0x123...}
│   ├── Face2 → {type: Client, zid: 0x456...}
│   └── ...
└── Routes (Computed Paths)
    ├── PubSub Routes
    ├── Query Routes
    └── Liveliness Routes
```

### Route Computation

Routes are computed lazily and cached:

```rust
pub struct Routes<T> {
    pub(crate) version: RoutesVersion,
    pub(crate) routers: RoutesVec<T>,
    pub(crate) peers: RoutesVec<T>,
    pub(crate) clients: RoutesVec<T>,
}

// Route computation with caching
fn get_route(resource: &Resource, face: &Face) -> Route {
    if let Some(cached) = route_cache.get(resource, face) {
        if cached.version == current_version {
            return cached;
        }
    }
    
    let route = compute_route(resource, face);
    route_cache.insert(resource, face, route);
    route
}
```

## Message Routing

### 1. Pub/Sub Routing

**Declaration Phase:**
1. Subscriber declares interest in key expression
2. Declaration propagates through network
3. Routing tables updated with subscriber location

**Data Phase:**
```mermaid
graph LR
    A[Publisher] -->|Publish| B[Local Router]
    B -->|Route Lookup| C[Routing Table]
    C -->|Matching Subscribers| D[Face List]
    D -->|Forward| E[Subscriber Faces]
```

### 2. Query Routing

**Query Propagation:**
1. Query initiated with target key expression
2. Router finds matching queryables
3. Query routed based on consolidation mode:
   - `None` - Route to all queryables
   - `Monotonic` - Aggregate replies
   - `Latest` - Only most recent reply

**Reply Routing:**
- Replies follow reverse path of query
- Consolidation points aggregate results

### 3. Liveliness Routing

Similar to pub/sub but for node presence:
- Liveliness tokens act like publications
- Subscribers monitor token presence
- Automatic cleanup on disconnection

## Gossip Protocol

Used for peer discovery and network topology dissemination:

```rust
pub struct GossipProtocol {
    enabled: bool,
    multihop: bool,
    target: WhatAmIMatcher,
    autoconnect: AutoConnect,
}

// Gossip message propagation
fn propagate_gossip(&self, msg: GossipMessage) {
    for peer in self.gossip_targets() {
        if should_forward_to(peer, msg) {
            peer.send(msg.clone());
        }
    }
}
```

## Route Optimization

### 1. Route Caching

- Routes cached per resource/face combination
- Cache invalidated on topology changes
- Version-based cache coherency

### 2. Batch Updates

Link-state updates are batched:
```rust
// Delayed tree computation
const TREE_COMPUTATION_DELAY: Duration = Duration::from_millis(100);

fn schedule_tree_computation(&mut self) {
    self.tree_computation_timer.set_after(TREE_COMPUTATION_DELAY);
}
```

### 3. Interceptor Support

Extensible routing through interceptors:
```rust
pub trait RoutingInterceptor {
    fn intercept_route(&self, 
        resource: &Resource, 
        route: &mut Route
    ) -> InterceptorResult;
}
```

## Failure Handling

### Link Failure Detection
- Keep-alive mechanism at transport layer
- Rapid failure detection through missing heartbeats
- Graceful shutdown notifications

### Route Recovery
1. Link failure detected
2. Face marked as down
3. Routes recomputed excluding failed face
4. Traffic rerouted automatically

### Split-Brain Prevention
- Unique node IDs prevent duplicate nodes
- Version vectors for consistent state
- Gossip protocol for partition detection

## Performance Characteristics

### Routing Complexity
- **Client**: O(1) - No routing computation
- **P2P**: O(n) - Linear in number of peers
- **Link-State**: O(n²) - Bellman-Ford algorithm
- **Router**: O(n²) - Full routing state

### Memory Usage
- Route caching trades memory for CPU
- Configurable cache sizes
- Automatic cache eviction on memory pressure

### Scalability Limits
- Tested with 1000+ nodes in link-state mode
- Router mode scales to 10,000+ clients
- Hierarchical deployments for larger scales