# HAT (Hierarchical Architecture Types) Implementations Study

## Objective

Provide detailed analysis of each HAT implementation in Zenoh, covering their routing strategies, target scenarios, and implementation details.

## Methodology

In-depth code analysis of all HAT implementations in `zenoh/src/net/routing/hat/`:
- Client HAT (`client/`)
- P2P Peer HAT (`p2p_peer/`)
- LinkState Peer HAT (`linkstate_peer/`)
- Router HAT (`router/`)

## Executive Summary

Zenoh provides four distinct HAT implementations, each optimized for different network topologies and scale requirements. From simple client forwarding to sophisticated hierarchical routing, each HAT targets specific deployment scenarios with appropriate algorithms and optimizations.

## 1. Client HAT Implementation

**Location**: `zenoh/src/net/routing/hat/client/mod.rs`

### 1.1 Routing Strategy

The Client HAT implements the simplest possible routing strategy:

```rust
impl HatBaseTrait for HatCode {
    fn ingress_filter(&self, _tables: &Tables, _face: &FaceState, _expr: &mut RoutingExpr<'_>) -> bool {
        true // Accept all incoming messages
    }

    fn egress_filter(&self, _tables: &Tables, src_face: &FaceState, out_face: &FaceState, _expr: &mut RoutingExpr<'_>) -> bool {
        src_face.id != out_face.id // Simple loop prevention
    }

    fn compute_data_route(&self, _tables: &Tables, _expr: &mut RoutingExpr<'_>, _source: NodeId, _source_type: WhatAmI) -> Arc<Route> {
        Arc::new(Route::default()) // No local routing decisions
    }
}
```

**Key Characteristics:**
- **No topology awareness**: No knowledge of network structure
- **Simple filtering**: Only prevents immediate loops
- **Delegation**: All routing decisions delegated to connected routers/peers
- **Minimal state**: No HAT-specific state required

### 1.2 Resource Management

```rust
impl HatPubSubTrait for HatCode {
    fn declare_subscription(&self, _hat_code: &(dyn HatTrait + Send + Sync), tables: &TablesLock, _face: &mut Arc<FaceState>, id: SubscriptionId, expr: &WireExpr<'_>, _sub_info: &SubscriptionInfo, _node_id: NodeId, _send_declare: &mut SendDeclare) {
        tables.tables.write().disable_all_routes(); // Invalidate routes on new subscription
    }

    fn forget_subscription(&self, _hat_code: &(dyn HatTrait + Send + Sync), tables: &TablesLock, _face: &mut Arc<FaceState>, id: SubscriptionId, _send_declare: &mut SendDeclare) {
        tables.tables.write().disable_all_routes(); // Invalidate routes on subscription removal
    }
}
```

**Resource Strategy:**
- **Route invalidation**: Clears all routes on any subscription change
- **No propagation**: Subscriptions not propagated to network
- **Local tracking**: Maintains local subscriptions only

### 1.3 Target Scenarios

**Ideal Use Cases:**
- Edge IoT devices with simple pub/sub needs
- Temporary applications connecting to Zenoh network
- Resource-constrained environments
- Applications that don't need routing capabilities

**Deployment Patterns:**
- Star topology: clients connect to central routers
- Single-hop: direct connection to router/peer required
- Stateless: clients can disconnect/reconnect frequently

**Performance Characteristics:**
- **Lowest overhead**: Minimal memory and CPU usage
- **Fast startup**: No topology discovery or route computation
- **Limited scalability**: Depends on infrastructure capacity
- **No resilience**: Single point of failure in connectivity

## 2. P2P Peer HAT Implementation

**Location**: `zenoh/src/net/routing/hat/p2p_peer/mod.rs`

### 2.1 Routing Strategy

P2P Peer HAT implements gossip-based routing:

```rust
impl HatBaseTrait for HatCode {
    fn ingress_filter(&self, tables: &Tables, face: &FaceState, _expr: &mut RoutingExpr<'_>) -> bool {
        if face.whatami == WhatAmI::Client {
            true
        } else {
            face.zid != tables.zid // Don't route back to sender
        }
    }

    fn egress_filter(&self, _tables: &Tables, src_face: &FaceState, out_face: &FaceState, _expr: &mut RoutingExpr<'_>) -> bool {
        src_face.id != out_face.id
            && match (src_face.whatami, out_face.whatami) {
                (WhatAmI::Client, WhatAmI::Client) => false, // No client-to-client routing
                _ => true,
            }
    }
}
```

### 2.2 Gossip Protocol Implementation

**Location**: `zenoh/src/net/routing/hat/p2p_peer/gossip.rs`

```rust
pub(super) struct Gossip {
    pub(super) network: Vec<ZenohIdProto>,        // Known peer network
    pub(super) locators: HashMap<ZenohIdProto, Vec<Locator>>, // Peer locators
}

impl Gossip {
    pub(super) fn recv_oam(&mut self, oam: Oam) {
        match oam.body {
            OamBody::LinkStateList(link_states) => {
                for link_state in link_states {
                    self.update_peer_locators(link_state.psid, link_state.locators);
                }
            }
        }
    }

    pub(super) fn send_gossip(&self, primitives: &Arc<dyn EPrimitives + Send + Sync>) {
        let oam = Oam {
            id: OamId::new(),
            body: OamBody::LinkStateList(self.build_link_states()),
        };
        primitives.send_oam(oam);
    }
}
```

**Gossip Features:**
- **Peer discovery**: Maintains list of known peers
- **Locator sharing**: Exchanges peer addresses for direct connection
- **Periodic updates**: Regular gossip message transmission
- **Failure detection**: Remove unreachable peers from network view

### 2.3 Subscription Propagation

```rust
impl HatPubSubTrait for HatCode {
    fn declare_subscription(&self, _hat_code: &(dyn HatTrait + Send + Sync), tables: &TablesLock, face: &mut Arc<FaceState>, id: SubscriptionId, expr: &WireExpr<'_>, sub_info: &SubscriptionInfo, node_id: NodeId, send_declare: &mut SendDeclare) {
        // Propagate subscription to all peer faces
        for (_, peer_face) in &tables.tables.read().faces {
            if peer_face.whatami == WhatAmI::Peer && peer_face.id != face.id {
                send_declare.push((peer_face.clone(), DeclareBody::DeclareSubscriber(DeclareSubscriber { ... })));
            }
        }
    }
}
```

### 2.4 Target Scenarios

**Ideal Use Cases:**
- Small to medium mesh networks (< 50 nodes)
- Dynamic environments with frequent joins/leaves
- Edge computing clusters
- Development and testing environments

**Performance Characteristics:**
- **Fast convergence**: Immediate propagation of local changes
- **Good resilience**: Multiple paths between nodes
- **Moderate overhead**: Gossip messages increase with network size
- **Limited scalability**: O(n²) message complexity in worst case

**Configuration Example:**
```json
{
  "mode": "peer",
  "routing": { "peer": { "mode": "peer_to_peer" } },
  "scouting": { 
    "gossip": { 
      "enabled": true,
      "multihop": true,
      "autoconnect": "peers"
    }
  }
}
```

## 3. LinkState Peer HAT Implementation

**Location**: `zenoh/src/net/routing/hat/linkstate_peer/mod.rs`

### 3.1 Network Topology Management

```rust
pub(super) struct Network {
    pub(super) name: String,
    pub(super) idx: NodeIndex,                    // Self index in graph
    pub(super) graph: StableUnGraph<Node, f64>,  // Network topology graph
    pub(super) distances: HashMap<NodeIndex, f64>, // Distances to all nodes
    pub(super) tree_computation_worker: Option<TreeComputationWorker>, // Background worker
}
```

**Graph Structure:**
- **Stable graph**: Uses petgraph with stable indices
- **Weighted edges**: Support for link weights and QoS
- **Node metadata**: Stores peer information and capabilities
- **Distance computation**: Precomputed shortest paths

### 3.2 SPF Tree Computation

```rust
impl Network {
    pub(super) fn compute_trees(&mut self) -> Vec<Vec<NodeIndex>> {
        let mut trees = Vec::new();
        
        for tree_root_idx in self.graph.node_indices() {
            if let Ok(paths) = petgraph::algo::bellman_ford(&self.graph, tree_root_idx) {
                let tree = self.build_tree_from_paths(tree_root_idx, &paths);
                trees.push(tree);
            }
        }
        
        trees
    }

    fn build_tree_from_paths(&self, root: NodeIndex, paths: &[Option<f64>]) -> Vec<NodeIndex> {
        // Build spanning tree from shortest path results
        // Ensures loop-free forwarding to all destinations
    }
}
```

**Tree Computation Features:**
- **Bellman-Ford algorithm**: Handles negative weights and detects cycles
- **Multiple trees**: One tree per potential root node
- **Background computation**: Asynchronous with 100ms batching delay
- **Incremental updates**: Only recompute when topology changes

### 3.3 Route Computation

```rust
impl HatBaseTrait for HatCode {
    fn compute_data_route(&self, tables: &Tables, expr: &mut RoutingExpr<'_>, source: NodeId, source_type: WhatAmI) -> Arc<Route> {
        let hat = get_hat!(tables);
        let network = &hat.network.as_ref().unwrap();
        
        let mut route = Route::default();
        
        // Find next hops for all subscribers
        for sub_face in self.find_subscribers(tables, expr) {
            if let Some(next_hop) = network.get_next_hop(sub_face.zid) {
                route.insert(next_hop, expr.clone());
            }
        }
        
        Arc::new(route)
    }
}
```

### 3.4 Target Scenarios

**Ideal Use Cases:**
- Large peer networks (50+ nodes)
- Stable topologies with infrequent changes
- Networks requiring optimal path selection
- Mission-critical applications needing guaranteed delivery

**Performance Characteristics:**
- **Optimal routing**: Always uses shortest paths
- **Higher resource usage**: Maintains complete topology
- **Computational overhead**: SPF calculations
- **Slower convergence**: Tree recomputation needed

**Configuration Example:**
```json
{
  "mode": "peer",
  "routing": {
    "peer": {
      "mode": "linkstate",
      "linkstate": {
        "transport_weights": [
          { "dst_zid": "1234567890abcdef", "weight": 100 }
        ]
      }
    }
  }
}
```

## 4. Router HAT Implementation

**Location**: `zenoh/src/net/routing/hat/router/mod.rs`

### 4.1 Dual Network Architecture

```rust
pub(super) struct HatTables {
    pub(super) router_subs: HashMap<Arc<Resource>, HashMap<ZenohIdProto, SubscriberId>>,
    pub(super) peer_subs: HashMap<Arc<Resource>, HashMap<ZenohIdProto, SubscriberId>>,
    pub(super) routers_net: Option<Network>,      // Router network topology
    pub(super) peers_net: Option<Network>,       // Peer network topology
    pub(super) shared_nodes: Vec<ZenohIdProto>,  // Nodes in both networks
    pub(super) router_peers_failover_brokering: bool, // Enable failover brokering
}
```

**Dual Network Support:**
- **Router network**: Full mesh of router nodes with link-state routing
- **Peer network**: Can use either link-state or P2P routing
- **Shared nodes**: Track nodes participating in both networks
- **Failover brokering**: Route between disconnected peer partitions

### 4.2 Router Election Algorithm

```rust
fn election_root(&self, tables: &Tables, key_expr: &keyexpr, mut roots: Vec<ZenohIdProto>) -> Vec<ZenohIdProto> {
    // Hash-based election to select master router for each key expression
    let hash = |zid: &ZenohIdProto| -> u64 {
        let mut hasher = DefaultHasher::new();
        key_expr.hash(&mut hasher);
        zid.hash(&mut hasher);
        hasher.finish()
    };
    
    roots.sort_by_key(|zid1| Reverse(hash(zid1)));
    roots
}
```

**Election Properties:**
- **Deterministic**: Same result on all routers
- **Load balanced**: Different expressions select different masters
- **Stable**: Election results don't change unless topology changes
- **Partition tolerant**: Works correctly during network splits

### 4.3 Failover Brokering

```rust
impl HatPubSubTrait for HatCode {
    fn declare_subscription(&self, hat_code: &(dyn HatTrait + Send + Sync), tables: &TablesLock, face: &mut Arc<FaceState>, id: SubscriptionId, expr: &WireExpr<'_>, sub_info: &SubscriptionInfo, node_id: NodeId, send_declare: &mut SendDeclare) {
        if self.router_peers_failover_brokering {
            // Check if this subscription needs brokering between peer partitions
            self.broker_peer_subscription(tables, expr, send_declare);
        }
    }
}
```

**Brokering Features:**
- **Partition detection**: Identify disconnected peer groups
- **Proxy routing**: Route messages between partitions via routers
- **Automatic recovery**: Remove brokering when peers reconnect
- **Selective brokering**: Only broker when necessary

### 4.4 Target Scenarios

**Ideal Use Cases:**
- Large-scale hierarchical deployments
- Multi-tier architectures (cloud-edge-device)
- Service provider networks
- Global IoT infrastructures

**Performance Characteristics:**
- **Highest scalability**: Supports thousands of nodes
- **Complex routing logic**: Highest resource requirements
- **Excellent fault tolerance**: Multiple redundancy mechanisms
- **Optimized for hierarchies**: Efficient multi-level routing

**Configuration Example:**
```json
{
  "mode": "router",
  "routing": {
    "router": {
      "peers_failover_brokering": true
    },
    "peer": {
      "mode": "linkstate",
      "linkstate": {
        "transport_weights": [
          { "dst_zid": "router1", "weight": 10 },
          { "dst_zid": "router2", "weight": 20 }
        ]
      }
    }
  }
}
```

## 5. HAT Selection Guidelines

### 5.1 Decision Matrix

| Scenario | Network Size | Topology | Recommended HAT |
|----------|-------------|----------|----------------|
| IoT Devices | Any | Star | Client |
| Small Mesh | < 20 nodes | Dynamic | P2P Peer |
| Medium Network | 20-100 nodes | Stable | LinkState Peer |
| Large Enterprise | 100+ nodes | Hierarchical | Router |
| Global Infrastructure | 1000+ nodes | Multi-tier | Router |

### 5.2 Performance Comparison

| HAT | Memory Usage | CPU Usage | Convergence Time | Scalability |
|-----|-------------|-----------|-----------------|-------------|
| Client | Minimal | Minimal | N/A | Limited |
| P2P Peer | Low | Low | Fast | Good |
| LinkState Peer | Medium | Medium | Medium | Excellent |
| Router | High | High | Slow | Outstanding |

### 5.3 Feature Comparison

| Feature | Client | P2P Peer | LinkState Peer | Router |
|---------|--------|----------|----------------|--------|
| Topology Awareness | None | Gossip | Full | Dual Network |
| Route Optimization | None | None | SPF | SPF + Election |
| Fault Tolerance | None | Good | Excellent | Outstanding |
| Failover Brokering | No | No | No | Yes |
| QoS Support | No | No | Yes | Yes |

## 6. Implementation Insights

### 6.1 Common Patterns

All HAT implementations share certain patterns:
- **Trait composition**: Implement multiple specialized traits
- **Type erasure**: Store HAT-specific state via `Box<dyn Any>`
- **Route caching**: Leverage common caching infrastructure
- **Filtering interface**: Provide ingress/egress filtering

### 6.2 Optimization Strategies

- **Lazy computation**: Compute routes only when needed
- **Background processing**: Use worker threads for expensive computations
- **Incremental updates**: Only recompute when necessary
- **Cache invalidation**: Efficient version-based invalidation

### 6.3 Future Enhancements

Potential improvements to HAT implementations:
1. **Adaptive algorithms**: Switch between strategies based on conditions
2. **Machine learning**: Optimize routing based on traffic patterns
3. **Geographic awareness**: Consider physical topology in routing decisions
4. **Energy efficiency**: Optimize for battery-powered devices

## Conclusion

Zenoh's HAT implementations provide a comprehensive range of routing strategies, from simple client forwarding to sophisticated hierarchical routing. Each implementation is carefully optimized for its target scenarios, providing the right balance of performance, scalability, and resource usage for different deployment contexts.