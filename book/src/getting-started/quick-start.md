# Quick Start

This guide will get you up and running with Zenoh in minutes. We'll start with the command-line tools to demonstrate core concepts, then show simple code examples.

## Prerequisites

Before starting, ensure you have [Zenoh installed](installation.md) on your system. You should be able to run:

```bash
zenohd --version
```

## Your First Zenoh Network

Let's start by running a Zenoh router, which will coordinate communication between applications.

### Step 1: Start a Zenoh Router

Open a terminal and start the Zenoh daemon:

```bash
zenohd
```

You should see output similar to:
```
[2024-01-01T12:00:00Z INFO  zenohd] Zenoh router v0.10.0 built with rustc 1.70.0
[2024-01-01T12:00:00Z INFO  zenoh::net::runtime] Using PID: 1234567890abcdef
[2024-01-01T12:00:00Z INFO  zenoh_transport::unicast::universal::rx] Starting RX transport task
[2024-01-01T12:00:00Z INFO  zenohd] zenohd listening on tcp/0.0.0.0:7447
```

Great! Your Zenoh router is now running and ready to handle data distribution.

## Basic Publish/Subscribe

Zenoh's publish/subscribe pattern allows applications to share data in real-time. Let's explore this with command-line tools.

### Step 2: Create a Subscriber

Open a **new terminal** (keep the router running) and create a subscriber:

```bash
# Subscribe to all data under the "demo" path
z_sub demo/**
```

The subscriber is now listening for any data published under the `demo/` hierarchy. The `**` wildcard means it will receive data from `demo/hello`, `demo/sensors/temperature`, or any other subpath.

### Step 3: Publish Some Data

Open a **third terminal** and publish some data:

```bash
# Publish a simple message
z_pub demo/hello "Hello, Zenoh!"

# Publish sensor data
z_pub demo/sensors/temperature "23.5"

# Publish JSON data
z_pub demo/device/status '{"online": true, "battery": 85}'
```

You should immediately see these messages appear in your subscriber terminal! This demonstrates Zenoh's real-time data distribution.

### Try Different Patterns

Experiment with different key expressions:

```bash
# Subscribe to only temperature data
z_sub demo/sensors/temperature

# Subscribe to all sensor data
z_sub demo/sensors/*

# Subscribe to everything
z_sub **
```

## Query/Queryable Pattern

Beyond pub/sub, Zenoh supports queries - a request/response pattern where you can ask for specific data.

### Step 4: Create a Queryable

A queryable responds to queries with data. Create one:

```bash
# Create a queryable that responds with system info
z_queryable demo/system/info
```

When prompted, enter some response data, like:
```
{"hostname": "my-laptop", "uptime": "2 days", "load": 0.5}
```

### Step 5: Send a Query

In another terminal, query for this data:

```bash
# Query for system info
z_get demo/system/info
```

You should receive the data you entered! Unlike pub/sub (which is continuous), queries are one-time requests that get immediate responses.

### Advanced Querying

Try querying with wildcards:

```bash
# Query all data under demo
z_get "demo/**"

# Query with a selector (more advanced pattern matching)
z_get "demo/sensors/*?(temperature>20)"
```

## Your First Code Example

Now let's write some actual code. Here's a simple Rust example:

### Publisher Example

Create a file called `publisher.rs`:

```rust
use zenoh::prelude::r#async::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a Zenoh session
    let session = zenoh::open(config::default()).res().await?;
    
    // Declare a publisher for our key expression
    let publisher = session
        .declare_publisher("demo/hello")
        .res()
        .await?;
    
    // Publish data every second
    for i in 1.. {
        let message = format!("Hello, Zenoh! #{}", i);
        println!("Publishing: {}", message);
        
        publisher.put(message).res().await?;
        sleep(Duration::from_secs(1)).await;
    }
    
    Ok(())
}
```

### Subscriber Example

Create a file called `subscriber.rs`:

```rust
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a Zenoh session
    let session = zenoh::open(config::default()).res().await?;
    
    // Declare a subscriber
    let subscriber = session
        .declare_subscriber("demo/hello")
        .callback(|sample| {
            println!(
                "Received ('{}': '{}')",
                sample.key_expr(),
                sample.value()
            );
        })
        .res()
        .await?;
    
    println!("Subscriber ready. Press Ctrl+C to exit.");
    
    // Keep the program running
    std::future::pending::<()>().await;
    Ok(())
}
```

### Running the Code

```bash
# Compile and run the subscriber in one terminal
rustc subscriber.rs --extern zenoh && ./subscriber

# Compile and run the publisher in another terminal  
rustc publisher.rs --extern zenoh && ./publisher
```

## Python Example

If you prefer Python, here's the same example:

### Python Publisher

```python
import zenoh
import time

# Open a Zenoh session
session = zenoh.open()

# Declare a publisher
publisher = session.declare_publisher("demo/hello")

# Publish data
for i in range(1, 100):
    message = f"Hello, Zenoh! #{i}"
    print(f"Publishing: {message}")
    publisher.put(message)
    time.sleep(1)

# Clean up
session.close()
```

### Python Subscriber

```python
import zenoh
import time

# Open a Zenoh session
session = zenoh.open()

# Define callback for received data
def listener(sample):
    print(f"Received ('{sample.key_expr}': '{sample.payload}')")

# Declare a subscriber
subscriber = session.declare_subscriber("demo/hello", listener)

print("Subscriber ready. Press Ctrl+C to exit.")

try:
    while True:
        time.sleep(1)
except KeyboardInterrupt:
    pass

# Clean up
session.close()
```

## Exploring the Network

Use Zenoh's introspection tools to explore your network:

```bash
# Show information about your Zenoh session
z_info

# List all routers in the network
z_info --routers

# List all peers
z_info --peers
```

## Key Concepts Demonstrated

In this quick start, you've experienced:

1. **Zero Configuration**: Zenoh works out of the box with no complex setup
2. **Unified API**: The same interface works for real-time streams and request/response
3. **Hierarchical Addressing**: Key expressions like `demo/sensors/temperature` create natural data organization
4. **Pattern Matching**: Wildcards (`*`, `**`) allow flexible data selection
5. **Language Agnostic**: The same concepts work across programming languages

## What's Happening Under the Hood?

When you ran these examples:

1. **Discovery**: Applications automatically discovered the Zenoh router
2. **Routing**: The router efficiently distributed data between publishers and subscribers
3. **Matching**: Key expressions were matched against subscriptions and queryables
4. **Transport**: Data flowed over the most appropriate network protocol (TCP by default)

## Different Deployment Modes

You can also run Zenoh without a separate router:

```bash
# Run in peer mode (applications talk directly to each other)
z_sub --mode=peer demo/**

# In another terminal
z_pub --mode=peer demo/hello "Direct P2P message!"
```

## Next Steps

Now that you've seen Zenoh in action, explore these topics:

1. **[Basic Concepts](basic-concepts.md)** - Understand key expressions, sessions, and data types
2. **[Programming Guide](../programming/sessions.md)** - Dive deeper into the APIs
3. **[Configuration](../programming/configuration.md)** - Customize Zenoh for your needs
4. **[Storage](../storage/overview.md)** - Add persistence to your data
5. **[Advanced Topics](../advanced/routing.md)** - Learn about routing and performance tuning

## Troubleshooting

**If examples don't work:**

1. Ensure `zenohd` is running
2. Check firewall settings (port 7447)
3. Try with explicit configuration:
   ```bash
   z_sub --listen=tcp/0.0.0.0:7448 demo/**
   z_pub --connect=tcp/127.0.0.1:7448 demo/hello "test"
   ```

**If you can't connect applications:**

1. Verify network connectivity
2. Check that applications are using the same Zenoh version
3. Use `z_info` to diagnose connectivity issues

## Summary

Congratulations! You've successfully:
- ✅ Started a Zenoh router
- ✅ Published and subscribed to data
- ✅ Used queries and queryables
- ✅ Written your first Zenoh applications
- ✅ Explored the command-line tools

Zenoh's power lies in its simplicity - the same simple primitives (put, get, subscribe) work whether you're building IoT sensors, distributed databases, or real-time robotics systems. The network automatically handles routing, discovery, and optimization, letting you focus on your application logic.
