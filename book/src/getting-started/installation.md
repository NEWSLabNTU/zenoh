# Installation

This chapter guides you through installing Zenoh on your system. There are several ways to get Zenoh up and running, depending on your needs and platform.

## System Requirements

### Supported Platforms
Zenoh supports the following platforms:
- **Linux**: Ubuntu 18.04+, Debian 10+, CentOS 7+, RHEL 7+
- **macOS**: macOS 10.15+ (Catalina or later)
- **Windows**: Windows 10, Windows Server 2019+
- **Embedded**: Various ARM and embedded Linux distributions

### Hardware Requirements
- **Minimum**: 64MB RAM, any modern CPU architecture
- **Recommended**: 256MB+ RAM for router deployments
- **Network**: Any network interface (Ethernet, Wi-Fi, cellular)

## Installation Methods

### Option 1: Pre-built Binaries (Recommended)

Pre-built binaries are the fastest way to get started with Zenoh.

#### Linux (Ubuntu/Debian)

Add the Zenoh repository and install:

```bash
# Add the Zenoh repository
echo "deb [trusted=yes] https://download.eclipse.org/zenoh/debian-repo/ /" | sudo tee -a /etc/apt/sources.list > /dev/null

# Update package list
sudo apt update

# Install Zenoh
sudo apt install zenoh
```

#### Linux (CentOS/RHEL/Fedora)

```bash
# Add the Zenoh repository
sudo tee /etc/yum.repos.d/zenoh.repo > /dev/null <<EOF
[zenoh]
name=Eclipse Zenoh Repository
baseurl=https://download.eclipse.org/zenoh/rpm-repo/
enabled=1
gpgcheck=0
EOF

# Install Zenoh
sudo yum install zenoh
# or for newer versions:
sudo dnf install zenoh
```

#### macOS

Using Homebrew (recommended):

```bash
# Add the Zenoh tap
brew tap eclipse-zenoh/homebrew-zenoh

# Install Zenoh
brew install zenoh
```

#### Windows

1. Visit the [Zenoh download page](https://download.eclipse.org/zenoh/)
2. Download the latest Windows release archive
3. Extract the archive to your desired location (e.g., `C:\Program Files\Zenoh`)
4. Add the Zenoh `bin` directory to your system PATH

Alternatively, using PowerShell:

```powershell
# Download and extract (replace VERSION with latest version)
Invoke-WebRequest -Uri "https://download.eclipse.org/zenoh/zenoh-VERSION-x86_64-pc-windows-msvc.zip" -OutFile "zenoh.zip"
Expand-Archive -Path "zenoh.zip" -DestinationPath "C:\Program Files\Zenoh"

# Add to PATH (requires administrator privileges)
$env:PATH += ";C:\Program Files\Zenoh\bin"
```

### Option 2: Building from Source

Building from source gives you access to the latest features and allows customization.

#### Prerequisites

First, install the Rust toolchain:

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment (or restart your shell)
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Clone and Build

```bash
# Clone the repository
git clone https://github.com/eclipse-zenoh/zenoh.git
cd zenoh

# Build with optimizations
cargo build --release --all-targets

# Install to your local cargo bin
cargo install --path zenohd --features default

# Verify installation
zenohd --version
```

#### Build Options

Zenoh supports various build features:

```bash
# Build with all features
cargo build --release --all-features

# Build with specific features
cargo build --release --features "transport_quic,transport_tls"

# Build for specific targets
cargo build --release --target x86_64-unknown-linux-musl
```

Common features:
- `transport_quic`: QUIC transport support
- `transport_tls`: TLS transport support
- `transport_vsock`: VSock transport support
- `shared-memory`: Shared memory transport
- `stats`: Runtime statistics collection

### Option 3: Docker

Run Zenoh in a Docker container:

```bash
# Pull the latest Zenoh image
docker pull eclipse/zenoh:latest

# Run a Zenoh router
docker run --rm -it -p 7447:7447/tcp -p 7447:7447/udp eclipse/zenoh:latest

# Run with custom configuration
docker run --rm -it -v /path/to/config:/config \
  eclipse/zenoh:latest -c /config/zenoh.json5
```

For development, you can also build your own image:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zenohd /usr/local/bin/
EXPOSE 7447
CMD ["zenohd"]
```

## Verification

After installation, verify that Zenoh is working correctly:

```bash
# Check version
zenohd --version

# Run help to see available options
zenohd --help

# Start a router in the foreground
zenohd

# In another terminal, test connectivity
zenoh_router_info
```

You should see output indicating that Zenoh is running and responding to queries.

## Language Bindings

### Python

Install the Python binding using pip:

```bash
# Install from PyPI
pip install eclipse-zenoh

# Verify installation
python -c "import zenoh; print(zenoh.__version__)"
```

### C/C++

Download the C bindings from the releases page:

```bash
# Download and extract (replace VERSION with latest)
wget https://download.eclipse.org/zenoh/zenoh-c-VERSION.tar.gz
tar -xzf zenoh-c-VERSION.tar.gz

# Install headers and libraries
sudo cp include/* /usr/local/include/
sudo cp lib/* /usr/local/lib/
sudo ldconfig
```

### Java

Add Zenoh to your Maven project:

```xml
<dependency>
    <groupId>io.zenoh</groupId>
    <artifactId>zenoh-java</artifactId>
    <version>VERSION</version>
</dependency>
```

Or for Gradle:

```gradle
implementation 'io.zenoh:zenoh-java:VERSION'
```

## Troubleshooting

### Common Issues

**Problem**: `zenohd: command not found`
**Solution**: Ensure the Zenoh binary is in your PATH. Check installation location and update PATH if necessary.

**Problem**: Permission denied when running zenohd
**Solution**: Check file permissions and ensure you have execute permissions:
```bash
chmod +x /path/to/zenohd
```

**Problem**: Network connectivity issues
**Solution**: Check firewall settings. Zenoh uses port 7447 by default:
```bash
# Allow Zenoh ports (Linux)
sudo ufw allow 7447

# Check if port is in use
netstat -tulpn | grep 7447
```

**Problem**: Rust compilation errors
**Solution**: Ensure you have the latest Rust toolchain:
```bash
rustup update
rustc --version  # Should be 1.70 or later
```

### Getting Help

If you encounter issues:

1. Check the [Zenoh documentation](https://zenoh.io/docs/)
2. Search [GitHub issues](https://github.com/eclipse-zenoh/zenoh/issues)
3. Ask questions on the [Discord community](https://discord.gg/vSDSpqnbkm)
4. Review logs with increased verbosity: `zenohd -v`

## Next Steps

Now that you have Zenoh installed, you're ready to:
- Follow the [Quick Start](quick-start.md) guide
- Learn about [Basic Concepts](basic-concepts.md)
- Explore the programming examples

The installation gives you access to:
- `zenohd` - The Zenoh router daemon
- Command-line tools for testing and debugging
- Language-specific libraries for your applications