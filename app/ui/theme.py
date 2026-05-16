"""
WCAG AAA palette + large typography.

Background `#0B1F3A` against text `#FFF8E7` measures ~13.8:1 contrast,
well above AAA's 7:1 threshold for normal text. Buttons use `#FFD166`
(amber) and `#06D6A0` (mint) for primary/positive states.
"""

from __future__ import annotations

from app.config import get_settings

COLORS = {
    "bg": "#0B1F3A",
    "bg_alt": "#102B53",
    "fg": "#FFF8E7",
    "fg_muted": "#D9CFB8",
    "accent": "#FFD166",
    "accent_text": "#0B1F3A",
    "success": "#06D6A0",
    "danger": "#EF476F",
    "info": "#118AB2",
    "outline": "#FFD166",
}


def base_stylesheet() -> str:
    settings = get_settings()
    font_size = settings.UI_FONT_SIZE
    return f"""
    QWidget {{
        background-color: {COLORS["bg"]};
        color: {COLORS["fg"]};
        font-family: "Segoe UI", "Helvetica Neue", Arial, sans-serif;
        font-size: {font_size}pt;
    }}

    QLabel#TitleLabel {{
        font-size: {font_size + 14}pt;
        font-weight: 600;
    }}

    QLabel#SubtitleLabel {{
        color: {COLORS["fg_muted"]};
        font-size: {font_size + 4}pt;
    }}

    QLabel#StepLabel {{
        font-size: {font_size + 6}pt;
        font-weight: 600;
    }}

    QLineEdit, QTextEdit {{
        background-color: {COLORS["bg_alt"]};
        color: {COLORS["fg"]};
        border: 3px solid {COLORS["fg_muted"]};
        border-radius: 16px;
        padding: 16px 22px;
        selection-background-color: {COLORS["accent"]};
        selection-color: {COLORS["accent_text"]};
        font-size: {font_size + 4}pt;
    }}

    QLineEdit:focus, QTextEdit:focus {{
        border-color: {COLORS["accent"]};
    }}

    QPushButton {{
        background-color: {COLORS["accent"]};
        color: {COLORS["accent_text"]};
        border: none;
        padding: 18px 28px;
        border-radius: 18px;
        font-weight: 700;
        min-height: 44px;
    }}

    QPushButton:hover {{
        background-color: #FFE08A;
    }}

    QPushButton:focus {{
        outline: none;
        border: 4px solid {COLORS["fg"]};
    }}

    QPushButton#MicButton {{
        background-color: {COLORS["info"]};
        color: {COLORS["fg"]};
    }}

    QPushButton#MicButton:checked, QPushButton#MicButton:pressed {{
        background-color: {COLORS["danger"]};
    }}

    QPushButton#YesButton {{
        background-color: {COLORS["success"]};
        color: {COLORS["accent_text"]};
        font-size: {font_size + 28}pt;
        min-height: 160px;
    }}

    QPushButton#NoButton {{
        background-color: {COLORS["danger"]};
        color: {COLORS["fg"]};
        font-size: {font_size + 28}pt;
        min-height: 160px;
    }}

    QFrame#FeedbackCard {{
        background-color: {COLORS["bg_alt"]};
        border-radius: 24px;
        padding: 24px;
    }}
    """
