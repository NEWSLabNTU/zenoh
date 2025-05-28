#!/usr/bin/env bash
#
# Standalone OpenTelemetry Collector runner for Zenoh experiments
# Reference shell script implementation
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Configuration defaults
COLLECTOR_NAME="${COLLECTOR_NAME:-zenoh-otel-collector}"
COLLECTOR_IMAGE="${COLLECTOR_IMAGE:-otel/opentelemetry-collector-contrib:latest}"
COLLECTOR_PORT="${COLLECTOR_PORT:-4317}"
COLLECTOR_HTTP_PORT="${COLLECTOR_HTTP_PORT:-4318}"
PROMETHEUS_PORT="${PROMETHEUS_PORT:-9090}"

# Script directory
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Functions
check_docker() {
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Error: Docker is not installed or not in PATH${NC}"
        return 1
    fi
    
    if ! docker info &> /dev/null; then
        echo -e "${RED}Error: Docker daemon is not running${NC}"
        return 1
    fi
    
    return 0
}

start_collector() {
    local config_file="${1:-$SCRIPT_DIR/otel-collector-config.yaml}"
    
    if [ ! -f "$config_file" ]; then
        echo -e "${RED}Error: Config file not found: $config_file${NC}"
        return 1
    fi
    
    # Check if already running
    if docker ps -q -f name="$COLLECTOR_NAME" | grep -q .; then
        echo -e "${YELLOW}Collector '$COLLECTOR_NAME' is already running${NC}"
        echo -n "Stop and restart? [y/N] "
        read -r response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
            docker stop "$COLLECTOR_NAME" > /dev/null 2>&1
            docker rm "$COLLECTOR_NAME" > /dev/null 2>&1
        else
            return 0
        fi
    fi
    
    echo -e "${BLUE}Starting OpenTelemetry Collector${NC}"
    echo "Config: $config_file"
    echo
    
    # Start collector
    echo -n "Starting collector..."
    if docker run -d \
        --name "$COLLECTOR_NAME" \
        -p "${COLLECTOR_PORT}:4317" \
        -p "${COLLECTOR_HTTP_PORT}:4318" \
        -p "${PROMETHEUS_PORT}:9090" \
        -v "$(realpath "$config_file"):/etc/otel-collector-config.yaml:ro" \
        "$COLLECTOR_IMAGE" \
        --config=/etc/otel-collector-config.yaml > /dev/null; then
        
        # Wait for container
        sleep 2
        
        # Verify it's running
        if docker ps -q -f name="$COLLECTOR_NAME" | grep -q .; then
            echo -e " ${GREEN}✓${NC}"
            echo
            echo -e "${GREEN}✓ Collector started successfully${NC}"
            echo
            echo "Services:"
            echo "  OTLP gRPC:  localhost:$COLLECTOR_PORT"
            echo "  OTLP HTTP:  localhost:$COLLECTOR_HTTP_PORT"
            echo "  Prometheus: http://localhost:$PROMETHEUS_PORT/metrics"
            echo
            echo -e "${DIM}View logs:${NC} docker logs -f $COLLECTOR_NAME"
            echo -e "${DIM}Stop:${NC} docker stop $COLLECTOR_NAME && docker rm $COLLECTOR_NAME"
            return 0
        fi
    fi
    
    echo -e " ${RED}✗${NC}"
    echo -e "${RED}Failed to start collector${NC}"
    echo "Check logs: docker logs $COLLECTOR_NAME"
    return 1
}

stop_collector() {
    echo -n "Stopping $COLLECTOR_NAME..."
    docker stop "$COLLECTOR_NAME" > /dev/null 2>&1
    docker rm "$COLLECTOR_NAME" > /dev/null 2>&1
    echo -e " ${GREEN}✓${NC}"
    echo -e "${GREEN}✓ Collector stopped${NC}"
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run standalone OpenTelemetry Collector for Zenoh experiments

Options:
  --config FILE    Path to collector config file
  --stop           Stop the running collector
  --help           Show this help message

Environment variables:
  COLLECTOR_NAME       Container name (default: zenoh-otel-collector)
  COLLECTOR_IMAGE      Docker image (default: otel/opentelemetry-collector-contrib:latest)
  COLLECTOR_PORT       OTLP gRPC port (default: 4317)
  COLLECTOR_HTTP_PORT  OTLP HTTP port (default: 4318)
  PROMETHEUS_PORT      Prometheus port (default: 9090)

Examples:
  # Start with default config
  $0

  # Start with custom config
  $0 --config /path/to/config.yaml

  # Stop collector
  $0 --stop
EOF
}

# Main
main() {
    local config_file=""
    local stop_mode=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --config)
                config_file="$2"
                shift 2
                ;;
            --stop)
                stop_mode=true
                shift
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Header
    echo -e "${BLUE}=== Zenoh OTLP Collector (Standalone) ===${NC}"
    echo
    
    # Check Docker
    if ! check_docker; then
        exit 1
    fi
    
    # Execute command
    if $stop_mode; then
        stop_collector
    else
        start_collector "$config_file"
    fi
}

# Run main
main "$@"
