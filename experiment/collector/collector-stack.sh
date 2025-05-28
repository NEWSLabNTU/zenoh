#!/usr/bin/env bash
#
# Full observability stack runner for Zenoh experiments
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
    
    # Check docker-compose
    if command -v docker-compose &> /dev/null; then
        COMPOSE_CMD="docker-compose"
    elif docker compose version &> /dev/null 2>&1; then
        COMPOSE_CMD="docker compose"
    else
        echo -e "${RED}Error: docker-compose is not available${NC}"
        echo "Install docker-compose or use Docker Desktop"
        return 1
    fi
    
    return 0
}

start_stack() {
    local compose_file="$SCRIPT_DIR/docker-compose.yaml"
    
    if [ ! -f "$compose_file" ]; then
        echo -e "${RED}Error: docker-compose.yaml not found in $SCRIPT_DIR${NC}"
        return 1
    fi
    
    echo -e "${BLUE}Starting Observability Stack${NC}"
    echo "Components: Collector, Jaeger, Prometheus, Grafana"
    echo
    
    # Start services
    echo -n "Starting services..."
    if cd "$SCRIPT_DIR" && $COMPOSE_CMD up -d > /dev/null 2>&1; then
        # Wait for services
        sleep 5
        
        # Check status
        if $COMPOSE_CMD ps 2>/dev/null | grep -q "Up"; then
            echo -e " ${GREEN}✓${NC}"
            echo
            echo -e "${GREEN}✓ Observability stack started successfully${NC}"
            echo
            echo "Services:"
            echo "┌─────────────────┬────────────────────────┬──────────────┐"
            echo "│ Service         │ URL                    │ Credentials  │"
            echo "├─────────────────┼────────────────────────┼──────────────┤"
            echo "│ OTLP Collector  │ localhost:4317 (gRPC)  │              │"
            echo "│                 │ localhost:4318 (HTTP)  │              │"
            echo "│ Jaeger UI       │ http://localhost:16686 │              │"
            echo "│ Prometheus      │ http://localhost:9090  │              │"
            echo "│ Grafana         │ http://localhost:3000  │ admin/admin  │"
            echo "└─────────────────┴────────────────────────┴──────────────┘"
            echo
            echo -e "${DIM}View logs:${NC} cd $SCRIPT_DIR && $COMPOSE_CMD logs -f"
            echo -e "${DIM}Stop stack:${NC} cd $SCRIPT_DIR && $COMPOSE_CMD down"
            return 0
        fi
    fi
    
    echo -e " ${RED}✗${NC}"
    echo -e "${RED}Some services failed to start${NC}"
    echo "Check status: cd $SCRIPT_DIR && $COMPOSE_CMD ps"
    return 1
}

stop_stack() {
    echo -n "Stopping services..."
    if cd "$SCRIPT_DIR" && $COMPOSE_CMD down > /dev/null 2>&1; then
        echo -e " ${GREEN}✓${NC}"
        echo -e "${GREEN}✓ Stack stopped${NC}"
        return 0
    else
        echo -e " ${RED}✗${NC}"
        echo -e "${RED}Error stopping stack${NC}"
        return 1
    fi
}

show_status() {
    echo -e "${BLUE}Checking service status...${NC}"
    echo
    cd "$SCRIPT_DIR" && $COMPOSE_CMD ps
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run full observability stack for Zenoh experiments

Options:
  --stop      Stop the running stack
  --status    Show status of services
  --help      Show this help message

Examples:
  # Start the stack
  $0

  # Check status
  $0 --status

  # Stop everything
  $0 --stop
EOF
}

# Main
main() {
    local mode="start"
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --stop)
                mode="stop"
                shift
                ;;
            --status)
                mode="status"
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
    echo -e "${BLUE}=== Zenoh Observability Stack ===${NC}"
    echo
    
    # Check prerequisites
    if ! check_docker; then
        exit 1
    fi
    
    # Execute command
    case $mode in
        stop)
            stop_stack
            ;;
        status)
            show_status
            ;;
        start)
            start_stack
            ;;
    esac
}

# Run main
main "$@"
