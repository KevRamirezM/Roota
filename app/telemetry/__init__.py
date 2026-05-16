"""
telemetry — structured local logging only. Nothing leaves the device.
"""

from app.telemetry.logger import configure_logging, get_logger

__all__ = ["configure_logging", "get_logger"]
