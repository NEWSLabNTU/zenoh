#!/usr/bin/env python3
"""
Pydantic models for Zenoh experiment configuration
"""

from typing import Optional, Literal
from pathlib import Path
from datetime import datetime
from pydantic import BaseModel, Field, computed_field, field_validator


class CommonConfig(BaseModel):
    """Common configuration shared across all profiles"""

    # Experiment identification
    experiment_id_prefix: str = Field(
        default="exp", description="Prefix for auto-generated experiment IDs"
    )
    experiment_base_dir: str = Field(
        default="/tmp/zenoh-experiments",
        description="Base directory for experiment data",
    )

    # Rust logging
    rust_log: str = Field(
        default="zenoh=debug,zenoh_transport=trace",
        description="Rust log level configuration",
    )

    # OpenTelemetry service
    otel_service_name: str = Field(
        default="zenoh-experiment", description="OpenTelemetry service name"
    )

    # Metadata capture
    enable_metadata_capture: bool = Field(
        default=True, description="Enable metadata file generation"
    )
    metadata_include_hostname: bool = Field(
        default=True, description="Include hostname in metadata"
    )
    metadata_include_command: bool = Field(
        default=True, description="Include command in metadata"
    )


class BandwidthConsciousConfig(BaseModel):
    """Configuration for bandwidth-constrained environments"""

    # Logging mode
    logging_mode: Literal["json-only"] = Field(
        default="json-only", description="Logging mode for bandwidth-conscious profile"
    )

    # Disable OpenTelemetry export
    otel_exporter_otlp_endpoint: Optional[str] = Field(
        default=None, description="OTLP endpoint (None for bandwidth-conscious mode)"
    )

    # Local logging
    local_log_format: Literal["json"] = Field(
        default="json", description="Format for local log files"
    )
    local_log_compression: bool = Field(default=False, description="Compress log files")

    # OpenTelemetry settings (disabled)
    otel_traces_exporter: Literal["none"] = Field(default="none")
    otel_metrics_exporter: Literal["none"] = Field(default="none")
    otel_logs_exporter: Literal["none"] = Field(default="none")


class EthernetConfig(BaseModel):
    """Configuration for high-bandwidth environments"""

    # Logging mode
    logging_mode: Literal["json+otlp"] = Field(
        default="json+otlp", description="Logging mode for ethernet profile"
    )

    # OpenTelemetry collector endpoint
    otel_exporter_otlp_endpoint: str = Field(
        default="http://localhost:4317", description="OTLP endpoint for trace export"
    )

    # Local logging
    local_log_format: Literal["json"] = Field(
        default="json", description="Format for local log files"
    )
    local_log_compression: bool = Field(default=True, description="Compress log files")

    # OpenTelemetry settings
    otel_traces_exporter: Literal["otlp"] = Field(default="otlp")
    otel_metrics_exporter: Literal["none"] = Field(default="none")
    otel_logs_exporter: Literal["none"] = Field(default="none")

    # Additional OTLP settings
    otel_exporter_otlp_protocol: Literal["grpc"] = Field(
        default="grpc", description="OTLP protocol"
    )
    otel_exporter_otlp_compression: Literal["gzip"] = Field(
        default="gzip", description="OTLP compression"
    )

    # Batch settings
    otel_bsp_schedule_delay: int = Field(
        default=5000, description="Batch span processor schedule delay (ms)"
    )
    otel_bsp_max_queue_size: int = Field(default=2048, description="Maximum queue size")
    otel_bsp_max_export_batch_size: int = Field(
        default=512, description="Maximum export batch size"
    )


class ExperimentProfile(BaseModel):
    """Complete experiment configuration with profile selection"""

    common: CommonConfig = Field(default_factory=CommonConfig)
    bandwidth_conscious: Optional[BandwidthConsciousConfig] = None
    ethernet: Optional[EthernetConfig] = None

    @field_validator("bandwidth_conscious", "ethernet")
    def at_least_one_profile(cls, v, info):
        """Ensure at least one profile is defined"""
        if (
            info.field_name == "ethernet"
            and v is None
            and info.data.get("bandwidth_conscious") is None
        ):
            raise ValueError("At least one profile must be defined")
        return v


class RuntimeConfig(BaseModel):
    """Runtime configuration combining static config with dynamic values"""

    # Profile selection
    profile: Literal["bandwidth-conscious", "ethernet"]

    # Base configuration
    config: ExperimentProfile

    # Dynamic values
    experiment_id: Optional[str] = None
    peer_id: Optional[str] = None

    def __init__(self, **data):
        super().__init__(**data)

        # Generate experiment ID if not provided
        if not self.experiment_id:
            prefix = self.config.common.experiment_id_prefix
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            self.experiment_id = f"{prefix}_{timestamp}"

    @computed_field
    @property
    def experiment_dir(self) -> Path:
        """Get experiment directory path"""
        base_dir = Path(self.config.common.experiment_base_dir)
        return base_dir / self.experiment_id

    @computed_field
    @property
    def profile_config(self) -> BandwidthConsciousConfig | EthernetConfig:
        """Get the active profile configuration"""
        if self.profile == "bandwidth-conscious":
            return self.config.bandwidth_conscious or BandwidthConsciousConfig()
        else:
            return self.config.ethernet or EthernetConfig()

    @computed_field
    @property
    def logging_mode(self) -> str:
        """Get logging mode from active profile"""
        return self.profile_config.logging_mode

    @computed_field
    @property
    def otlp_endpoint(self) -> Optional[str]:
        """Get OTLP endpoint from active profile"""
        return self.profile_config.otel_exporter_otlp_endpoint

    @computed_field
    @property
    def rust_log(self) -> str:
        """Get Rust log configuration"""
        return self.config.common.rust_log

    @computed_field
    @property
    def enable_metadata_capture(self) -> bool:
        """Check if metadata capture is enabled"""
        return self.config.common.enable_metadata_capture

    @computed_field
    @property
    def metadata_include_hostname(self) -> bool:
        """Check if hostname should be included in metadata"""
        return self.config.common.metadata_include_hostname

    @computed_field
    @property
    def metadata_include_command(self) -> bool:
        """Check if command should be included in metadata"""
        return self.config.common.metadata_include_command

    @computed_field
    @property
    def log_file(self) -> Path:
        """Get log file path"""
        if not self.peer_id:
            raise ValueError("Peer ID not set")
        return self.experiment_dir / f"{self.peer_id}.log"

    def is_bandwidth_constrained(self) -> bool:
        """Check if running in bandwidth-constrained mode"""
        return self.logging_mode == "json-only"

    def should_compress_logs(self) -> bool:
        """Check if logs should be compressed"""
        return self.profile_config.local_log_compression

    def get_environment(self) -> dict[str, str]:
        """Get environment variables for the experiment"""
        env = {}

        # Standard Rust/Zenoh variables
        env["RUST_LOG"] = self.rust_log

        # OpenTelemetry variables
        if self.otlp_endpoint:
            env["OTEL_EXPORTER_OTLP_ENDPOINT"] = self.otlp_endpoint

        env["OTEL_SERVICE_NAME"] = self.config.common.otel_service_name
        env["OTEL_TRACES_EXPORTER"] = self.profile_config.otel_traces_exporter
        env["OTEL_METRICS_EXPORTER"] = self.profile_config.otel_metrics_exporter
        env["OTEL_LOGS_EXPORTER"] = self.profile_config.otel_logs_exporter

        # Additional OTLP settings for ethernet profile
        if not self.is_bandwidth_constrained():
            eth_config = self.profile_config
            env["OTEL_EXPORTER_OTLP_PROTOCOL"] = eth_config.otel_exporter_otlp_protocol
            env["OTEL_EXPORTER_OTLP_COMPRESSION"] = (
                eth_config.otel_exporter_otlp_compression
            )
            env["OTEL_BSP_SCHEDULE_DELAY"] = str(eth_config.otel_bsp_schedule_delay)
            env["OTEL_BSP_MAX_QUEUE_SIZE"] = str(eth_config.otel_bsp_max_queue_size)
            env["OTEL_BSP_MAX_EXPORT_BATCH_SIZE"] = str(
                eth_config.otel_bsp_max_export_batch_size
            )

        # Experiment metadata
        env["EXPERIMENT_ID"] = self.experiment_id
        env["ZENOH_EXPERIMENT_PROFILE"] = self.profile

        return env
