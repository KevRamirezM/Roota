"""
telemetry.logger — Centralised loguru configuration.

Call `setup_logging()` once at startup to configure rotating file
and console sinks. All other modules should import `logger` from here.
"""

import sys
from pathlib import Path

from loguru import logger

from app.config.settings import settings


def setup_logging() -> None:
    """Configure loguru sinks based on current settings."""
    logger.remove()

    # Console sink
    logger.add(
        sys.stderr,
        level=settings.log_level,
        format="<green>{time:HH:mm:ss}</green> | <level>{level: <8}</level> | {message}",
        colorize=True,
    )

    # Rotating file sink
    log_dir = Path(settings.log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)
    logger.add(
        log_dir / "roota_{time:YYYY-MM-DD}.log",
        level=settings.log_level,
        rotation="10 MB",
        retention="7 days",
        compression="zip",
        encoding="utf-8",
    )
