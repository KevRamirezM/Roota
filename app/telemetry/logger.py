"""
Local-only logging configuration backed by loguru.

A rotating file sink in `LOG_DIR/roota.log` plus a coloured console
sink. We never ship logs anywhere; the privacy contract is enforced
by simply not having any network sink.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

from loguru import logger as _logger

from app.config import get_settings

_configured = False


def configure_logging() -> None:
    """Idempotently configure loguru sinks from Settings."""
    global _configured
    if _configured:
        return

    settings = get_settings()

    _logger.remove()

    _logger.add(
        sys.stderr,
        level=settings.LOG_LEVEL,
        colorize=True,
        format=(
            "<green>{time:HH:mm:ss}</green> | "
            "<level>{level: <8}</level> | "
            "<cyan>{name}</cyan> - <level>{message}</level>"
        ),
    )

    log_dir = Path(settings.LOG_DIR)
    log_dir.mkdir(parents=True, exist_ok=True)
    _logger.add(
        log_dir / "roota.log",
        level=settings.LOG_LEVEL,
        rotation="5 MB",
        retention=5,
        encoding="utf-8",
        enqueue=True,
        backtrace=False,
        diagnose=False,
    )

    _configured = True


def get_logger(name: str | None = None) -> Any:
    """Return a loguru logger bound to the given module name."""
    if not _configured:
        configure_logging()
    return _logger.bind(name=name) if name else _logger
