# Zenoh Experiment Framework

A comprehensive framework for running and monitoring Zenoh experiments with OpenTelemetry integration.

## Installation

This project uses [Rye](https://rye-up.com/) for dependency management:

```bash
# Install dependencies
rye sync

# Activate the virtual environment
source .venv/bin/activate
```

## Usage

### Running Experiments

The framework supports two profiles:
- **bandwidth-conscious** (default): JSON logging only, no network export
- **ethernet**: Full OpenTelemetry export to collector

```bash
# Run publisher with default profile
zenoh-pub

# Run subscriber with ethernet profile
zenoh-sub --profile ethernet

# With Zenoh arguments
zenoh-pub -- -k demo/example -v "Hello World"
zenoh-sub -- -k demo/example
```

### Collector Management

The collector uses shell scripts for Docker management:

```bash
# Start standalone collector
./collector/collector-standalone.sh

# Start full observability stack (Collector + Jaeger + Grafana)
./collector/collector-stack.sh

# Check status
./collector/collector-stack.sh --status

# Stop services
./collector/collector-standalone.sh --stop
./collector/collector-stack.sh --stop
```

## Configuration

Configuration is managed via `config.yaml`:

- **Common settings**: Shared across all profiles
- **bandwidth_conscious profile**: For low-bandwidth environments
- **ethernet profile**: For full observability

## Project Structure

```
experiment/
├── config.yaml            # Main configuration file
├── config/                # Configuration management
│   ├── models.py          # Pydantic models
│   └── experiment_config.py
├── pub/                   # Publisher scripts
│   ├── pub.py
│   └── peer-metadata.json.in
├── sub/                   # Subscriber scripts
│   ├── sub.py
│   └── peer-metadata.json.in
└── collector/             # Observability stack
    ├── collector-standalone.sh
    ├── collector-stack.sh
    ├── docker-compose.yaml
    └── *.yaml             # Various config files
```

## Development

```bash
# Install in development mode
rye sync

# Run tests (when available)
rye test

# Format code
rye fmt

# Lint code
rye lint
```