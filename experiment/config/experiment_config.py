#!/usr/bin/env python3
"""
Experiment configuration loader for Zenoh experiments
Uses Pydantic for declarative configuration management
"""

import yaml
from pathlib import Path
from typing import Optional

from .models import ExperimentProfile, RuntimeConfig


class ExperimentConfig:
    """Configuration loader for Zenoh experiments"""

    def __init__(self, config_file: str):
        """
        Initialize experiment configuration

        Args:
            config_file: Path to configuration file
        """
        # Set config file path
        self.config_file = Path(config_file)

        # Load and validate configuration (this will set self.profile)
        self._runtime_config = self._load_config()

        # Create experiment directory
        self.experiment_dir.mkdir(parents=True, exist_ok=True)

    def _load_config(self) -> RuntimeConfig:
        """Load configuration from YAML file"""
        if not self.config_file.exists():
            raise FileNotFoundError(f"Configuration file not found: {self.config_file}")

        # Load YAML
        with open(self.config_file, "r") as f:
            config_data = yaml.safe_load(f)

        # Extract profile from config data
        self.profile = config_data.get("profile", "bandwidth-conscious")

        # Create ExperimentProfile from YAML data (excluding the top-level profile field)
        profile_data = {k: v for k, v in config_data.items() if k != "profile"}
        experiment_profile = ExperimentProfile(**profile_data)

        # Create RuntimeConfig with selected profile
        runtime_config = RuntimeConfig(profile=self.profile, config=experiment_profile)

        return runtime_config

    # Proxy properties to RuntimeConfig
    @property
    def experiment_id(self) -> str:
        return self._runtime_config.experiment_id

    @experiment_id.setter
    def experiment_id(self, value: str):
        self._runtime_config.experiment_id = value

    @property
    def peer_id(self) -> Optional[str]:
        return self._runtime_config.peer_id

    @peer_id.setter
    def peer_id(self, value: str):
        self._runtime_config.peer_id = value

    @property
    def experiment_dir(self) -> Path:
        return self._runtime_config.experiment_dir

    @property
    def logging_mode(self) -> str:
        return self._runtime_config.logging_mode

    @property
    def otlp_endpoint(self) -> Optional[str]:
        return self._runtime_config.otlp_endpoint

    @property
    def rust_log(self) -> str:
        return self._runtime_config.rust_log

    @property
    def enable_metadata_capture(self) -> bool:
        return self._runtime_config.enable_metadata_capture

    @property
    def metadata_include_hostname(self) -> bool:
        return self._runtime_config.metadata_include_hostname

    @property
    def metadata_include_command(self) -> bool:
        return self._runtime_config.metadata_include_command

    @property
    def log_file(self) -> Path:
        return self._runtime_config.log_file

    def is_bandwidth_constrained(self) -> bool:
        """Check if running in bandwidth-constrained mode"""
        return self._runtime_config.is_bandwidth_constrained()

    def should_compress_logs(self) -> bool:
        """Check if logs should be compressed"""
        return self._runtime_config.should_compress_logs()

    def get_environment(self) -> dict[str, str]:
        """Get environment variables for the experiment"""
        return self._runtime_config.get_environment()

    def export_config(self, output_file: Optional[Path] = None) -> dict:
        """
        Export current configuration as dictionary

        Args:
            output_file: Optional file to save configuration to

        Returns:
            Configuration dictionary
        """
        config_dict = self._runtime_config.model_dump()

        if output_file:
            with open(output_file, "w") as f:
                yaml.dump(config_dict, f, default_flow_style=False)

        return config_dict
