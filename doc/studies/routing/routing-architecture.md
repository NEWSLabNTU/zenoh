# Zenoh Routing Architecture Study

## Objective

Analyze the architectural design of Zenoh's routing system, focusing on the type system, trait hierarchy, and core abstractions that enable pluggable routing strategies.

## Methodology

Code analysis of the routing system in `zenoh/src/net/routing/`, examining:
1. Core types and trait definitions
2. HAT (Hierarchical Architecture Types) implementation
3. Face and Resource abstractions
4. Message routing pipeline
5. Memory management and thread safety patterns

## Executive Summary

Zenoh's routing architecture is built around a sophisticated trait system that enables pluggable routing strategies through the HAT (Hierarchical Architecture Types) abstraction. The design separates routing policy from mechanism, allowing different algorithms (client, peer, router) to plug into a common infrastructure.

## 1. Core Architectural Components

### 1.1 Central Tables

The `Tables` struct serves as the central routing state manager:

```rust
pub struct Tables {
    pub(crate) zid: ZenohIdProto,           // Node identifier
    pub(crate) whatami: WhatAmI,            // Node type (Client/Peer/Router)
    pub(crate) root_res: Arc<Resource>,     // Root of resource hierarchy
    pub(crate) faces: HashMap<usize, Arc<FaceState>>, // Active connections
    pub(crate) mcast_groups: Vec<Arc<FaceState>>,     // Multicast groups
    pub(crate) hat: Box<dyn Any + Send + Sync>,      // HAT-specific state
    pub(crate) routes_version: RoutesVersion,         // Cache invalidation
    // ... interceptors, queries, interests
}
```

**Key Design Decisions:**
- **Type-erased HAT state**: Allows strategy-specific data without core changes
- **Arc-based sharing**: Enables efficient face and resource sharing
- **Versioned routing**: Global version for efficient cache invalidation
- **Hierarchical resources**: Tree structure for key expression matching

### 1.2 Face Abstraction

Faces represent connections to other Zenoh nodes:

```rust
pub struct FaceState {
    pub(crate) id: usize,                    // Unique face identifier
    pub(crate) zid: ZenohIdProto,           // Remote node identifier
    pub(crate) whatami: WhatAmI,            // Remote node type
    pub(crate) primitives: Arc<dyn EPrimitives + Send + Sync>, // Send interface
    pub(crate) local_mappings: HashMap<ExprId, Arc<Resource>>,  // Expression mappings
    pub(crate) remote_mappings: HashMap<ExprId, Arc<Resource>>, // Remote mappings
    pub(crate) hat: Box<dyn Any + Send + Sync>, // HAT-specific state
    // ... interests, queries, interceptors
}
```

**Capabilities:**
- **Expression mapping**: Converts between wire IDs and full expressions
- **Interest tracking**: Manages subscriptions and queryables
- **Query management**: Handles pending queries and timeouts
- **HAT integration**: Carries routing strategy-specific state

### 1.3 Resource Hierarchy

Resources form a tree matching key expression hierarchy:

```rust
pub struct Resource {
    pub(crate) parent: Option<Arc<Resource>>,    // Parent in hierarchy
    pub(crate) expr: String,                     // Key expression segment
    pub(crate) suffix: usize,                    // Suffix start in expr
    pub(crate) children: SingleOrBoxHashSet<Child>, // Child resources
    pub(crate) context: Option<Box<ResourceContext>>, // Routing context
    pub(crate) session_ctxs: HashMap<usize, Arc<SessionContext>>, // Per-face contexts
}
```

**Resource Context:**
```rust
pub(crate) struct ResourceContext {
    pub(crate) matches: Vec<Weak<Resource>>,      // Matching resources
    pub(crate) hat: Box<dyn Any + Send + Sync>,  // HAT-specific state
    pub(crate) data_routes: RwLock<DataRoutes>,  // Cached data routes
    pub(crate) query_routes: RwLock<QueryRoutes>, // Cached query routes
}
```

## 2. HAT (Hierarchical Architecture Types) System

### 2.1 Trait Hierarchy

The HAT system enables pluggable routing through a trait hierarchy:

```rust
pub(crate) trait HatTrait: 
    HatBaseTrait + HatInterestTrait + HatPubSubTrait + HatQueriesTrait + HatTokenTrait
{}
```

**Component Traits:**

1. **HatBaseTrait**: Core infrastructure
   - Node lifecycle management
   - Route computation interface
   - Ingress/egress filtering

2. **HatPubSubTrait**: Publish/Subscribe routing
   - Subscription management
   - Data route computation
   - Interest propagation

3. **HatQueriesTrait**: Query/Reply routing
   - Query route computation
   - Queryable management
   - Reply aggregation

4. **HatTokenTrait**: Liveliness token routing
   - Token propagation
   - Liveness detection
   - Cleanup on disconnection

### 2.2 Strategy Selection

HAT implementation is selected based on node type and configuration:

```rust
pub(crate) fn new_hat(whatami: WhatAmI, config: &Config) -> Box<dyn HatTrait + Send + Sync> {
    match whatami {
        WhatAmI::Client => Box::new(client::HatCode {}),
        WhatAmI::Peer => {
            if unwrap_or_default!(config.routing().peer().mode()) == *"linkstate" {
                Box::new(linkstate_peer::HatCode {})
            } else {
                Box::new(p2p_peer::HatCode {})
            }
        }
        WhatAmI::Router => Box::new(router::HatCode {}),
    }
}
```

## 3. Routing Pipeline Architecture

### 3.1 Message Flow

The routing pipeline processes messages through several stages:

```
Transport → Face → Dispatcher → HAT → Route Cache → Forwarding
```

1. **Message Reception**: Transport layer delivers message to Face
2. **Context Creation**: Wrap message in RoutingContext
3. **Expression Resolution**: Resolve wire expressions to Resources
4. **HAT Processing**: Route through strategy-specific logic
5. **Route Computation**: Get or compute routes for target expression
6. **Filtering**: Apply ingress/egress filters
7. **Forwarding**: Send to target faces

### 3.2 RoutingContext

RoutingContext carries metadata through the pipeline:

```rust
pub(crate) struct RoutingContext<Msg> {
    pub(crate) msg: Msg,                     // The message being routed
    pub(crate) inface: OnceCell<Face>,       // Source face
    pub(crate) outface: OnceCell<Face>,      // Destination face
    pub(crate) prefix: OnceCell<Arc<Resource>>, // Resolved prefix
    pub(crate) full_expr: OnceCell<String>,  // Complete expression
}
```

**Lazy Evaluation Benefits:**
- **Performance**: Avoid unnecessary computations
- **Memory**: Don't store unused values
- **Flexibility**: Context can be partially populated

## 4. Route Computation and Caching

### 4.1 Route Caching Strategy

```rust
pub(crate) struct Routes<T> {
    routers: Vec<Option<T>>,    // Routes for router contexts
    peers: Vec<Option<T>>,      // Routes for peer contexts
    clients: Vec<Option<T>>,    // Routes for client contexts
    version: u64,               // Version for cache invalidation
}
```

**Cache Management:**
- **Context-specific**: Separate routes for different node types
- **Versioned**: Global version invalidates all cached routes
- **Lazy**: Routes computed on first access
- **Resource-local**: Each resource maintains its own cache

### 4.2 Route Computation Process

```rust
fn get_data_route(
    hat_code: &(dyn HatTrait + Send + Sync),
    tables: &Tables,
    face: &FaceState,
    res: &Option<Arc<Resource>>,
    expr: &mut RoutingExpr,
    routing_context: NodeId,
) -> Arc<Route> {
    let local_context = hat_code.map_routing_context(tables, face, routing_context);
    let compute_route = || hat_code.compute_data_route(tables, expr, local_context, face.whatami);
    get_or_set_route(data_routes, tables.routes_version, face.whatami, local_context, compute_route)
}
```

## 5. Memory Management and Thread Safety

### 5.1 Reference Counting Strategy

**Arc Usage:**
- `Arc<Resource>`: Shared ownership across faces and routing tables
- `Arc<FaceState>`: Shared face state across routing components
- `Arc<Route>`: Shared route results to avoid recomputation

**Weak References:**
- `Weak<Resource>`: Break cycles in resource matching relationships
- Prevent memory leaks in complex graph structures

### 5.2 Locking Strategy

```rust
pub struct TablesLock {
    pub tables: RwLock<Tables>,              // Main routing state
    pub(crate) hat_code: Box<dyn HatTrait + Send + Sync>, // HAT implementation
    pub(crate) ctrl_lock: Mutex<()>,        // Control plane serialization
    pub queries_lock: RwLock<()>,           // Query operation lock
}
```

**Concurrency Patterns:**
- **Reader-writer locks**: Allow concurrent route lookups
- **Control lock**: Serialize topology changes
- **Lock-free paths**: Route cache lookups without locks
- **ArcSwap**: Lock-free interceptor updates

### 5.3 Resource Cleanup

```rust
pub fn clean(res: &mut Arc<Resource>) {
    if Arc::strong_count(res) <= 3 && res.children.is_empty() {
        // Safe to clean - only held by minimal references
        if let Some(parent) = res.parent.as_ref() {
            parent.children.remove(&res.expr);
            clean(parent); // Recursive cleanup
        }
    }
}
```

**Cleanup Triggers:**
- Face disconnection
- Subscription withdrawal
- Query completion
- Interest expiration

## 6. Key Design Benefits

### 6.1 Pluggability

The HAT system enables complete isolation of routing strategies:
- **Interface isolation**: Common trait interface for all strategies
- **State isolation**: Type-erased strategy-specific state
- **Algorithm isolation**: No coupling between different approaches

### 6.2 Performance

Multiple optimizations ensure high performance:
- **Route caching**: Avoid recomputation of stable routes
- **Lazy evaluation**: Compute only what's needed
- **Lock-free paths**: Fast path for cached route lookups
- **Hierarchical structure**: Efficient key expression matching

### 6.3 Scalability

Architecture scales across different deployment sizes:
- **Memory efficiency**: Weak references prevent leaks
- **Cache efficiency**: Version-based invalidation
- **Computation efficiency**: Context-specific route separation
- **Network efficiency**: Strategy-appropriate algorithms

### 6.4 Safety

Strong safety guarantees throughout:
- **Memory safety**: Rust ownership and reference counting
- **Thread safety**: Careful lock ordering and lock-free paths
- **Type safety**: Compile-time prevention of invalid operations
- **Protocol safety**: HAT interface prevents invalid routing decisions

## 7. Trade-offs and Considerations

### 7.1 Complexity vs Flexibility

**Benefits:**
- Complete routing strategy isolation
- Easy addition of new strategies
- Optimal algorithms for each scenario

**Costs:**
- Complex trait hierarchy
- Type erasure reduces compile-time checking
- Increased memory usage for multiple caches

### 7.2 Performance vs Generality

**Benefits:**
- High performance through specialization
- Efficient caching and lazy evaluation
- Lock-free fast paths

**Costs:**
- Memory overhead for caching
- Code complexity for optimization
- Potential for cache invalidation storms

## 8. Future Directions

Potential improvements to the routing architecture:

1. **Cache Optimization**: More sophisticated cache replacement policies
2. **Lock Reduction**: Further reduction of locking in hot paths
3. **Memory Optimization**: Better memory layout for cache efficiency
4. **Algorithm Improvements**: More efficient route computation algorithms
5. **Observability**: Better metrics and debugging for routing decisions

## Conclusion

Zenoh's routing architecture successfully balances performance, flexibility, and safety through a sophisticated design that separates policy from mechanism. The HAT system enables pluggable routing strategies while maintaining a common, optimized infrastructure for high-performance message routing in distributed systems.