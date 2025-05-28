#!/usr/bin/env python3
"""
Zenoh Subscriber Script
Runs the Zenoh subscriber example with configured logging
"""

import os
import sys
import json
import subprocess
import argparse
from datetime import datetime
from pathlib import Path

# Add parent directory to path for config module
sys.path.insert(0, str(Path(__file__).parent.parent))

from config.experiment_config import ExperimentConfig


def setup_logging(config: ExperimentConfig, peer_type: str):
    """Setup logging configuration for the peer"""
    peer_id = f"{peer_type}_{os.uname().nodename}_{os.getpid()}"
    config.peer_id = peer_id

    print("=== Zenoh Subscriber Configuration ===")
    print(f"Profile: {config.profile}")
    print(f"Logging mode: {config.logging_mode}")

    if config.is_bandwidth_constrained():
        print("Running in bandwidth-constrained mode (JSON logs only)")
        print(f"Logs will be saved to: {config.log_file}")
    else:
        print(f"Running with OpenTelemetry export to: {config.otlp_endpoint}")
        print(f"Local logs also saved to: {config.log_file}")

    print(f"Experiment ID: {config.experiment_id}")
    print(f"Peer ID: {peer_id}")
    print("======================================")

    return peer_id


def create_metadata(config: ExperimentConfig, peer_id: str, command: list):
    """Create metadata file for this peer"""
    metadata = {
        "experiment_id": config.experiment_id,
        "peer_id": peer_id,
        "hostname": os.uname().nodename if config.metadata_include_hostname else "",
        "start_time": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "profile": config.profile,
        "logging_mode": config.logging_mode,
        "otlp_endpoint": config.otlp_endpoint if config.otlp_endpoint else None,
        "rust_log": config.rust_log,
        "command": " ".join(command) if config.metadata_include_command else "",
    }

    metadata_file = config.experiment_dir / f"{peer_id}.metadata.json"
    with open(metadata_file, "w") as f:
        json.dump(metadata, f, indent=2)

    return metadata_file


def build_zenoh(features: list):
    """Build Zenoh with specified features"""
    print("Building Zenoh with OpenTelemetry support...")
    cmd = ["cargo", "build", "--example", "z_sub"]

    if features:
        cmd.extend(["--features", ",".join(features)])

    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Build failed:\n{result.stderr}")
        sys.exit(1)

    print("Build successful")


def run_subscriber(config: ExperimentConfig, args: list):
    """Run the Zenoh subscriber with logging"""
    # Build command
    cmd = [
        "cargo",
        "run",
        "--features",
        "zenoh-util/opentelemetry",
        "--example",
        "z_sub",
        "--",
    ] + args

    # Setup environment
    env = os.environ.copy()
    env.update(config.get_environment())

    # Open log file
    log_file = open(config.log_file, "w")

    try:
        # Run with both stdout and stderr redirected to log file and console
        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env=env,
            text=True,
            bufsize=1,
        )

        # Stream output to both console and log file
        for line in process.stdout:
            print(line, end="")
            log_file.write(line)
            log_file.flush()

        process.wait()
        return process.returncode

    except KeyboardInterrupt:
        print("\nInterrupted by user")
        process.terminate()
        return 130
    finally:
        log_file.close()

        # Compress log if configured
        if config.should_compress_logs():
            print(f"Compressing log file...")
            subprocess.run(["gzip", str(config.log_file)])


def main():
    parser = argparse.ArgumentParser(
        description="Run Zenoh subscriber with experiment logging",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run with default bandwidth-conscious profile
  ./sub.py

  # Run with ethernet profile
  ./sub.py --profile ethernet

  # Pass arguments to z_sub
  ./sub.py -- -k demo/example
""",
    )

    parser.add_argument(
        "--config",
        type=Path,
        default=Path(__file__).parent.parent / "config.yaml",
        help="Path to configuration file",
    )

    # All remaining arguments are passed to z_sub
    parser.add_argument(
        "sub_args", nargs=argparse.REMAINDER, help="Arguments to pass to z_sub"
    )

    args = parser.parse_args()

    # Remove -- from sub_args if present
    if args.sub_args and args.sub_args[0] == "--":
        args.sub_args = args.sub_args[1:]

    # Load configuration from file
    config = ExperimentConfig(config_file=str(args.config))

    # Setup logging
    peer_id = setup_logging(config, "subscriber")

    # Create metadata
    if config.enable_metadata_capture:
        create_metadata(config, peer_id, ["z_sub"] + args.sub_args)

    # Build Zenoh
    build_zenoh(["zenoh-util/opentelemetry"])

    # Run subscriber
    return run_subscriber(config, args.sub_args)


if __name__ == "__main__":
    sys.exit(main())
