"""
app.ui.main — Application entry point.

Launches the Roota PySide6 window and wires up the core subsystems.
"""

import sys


def main() -> None:
    # PySide6 import is deferred so the module can be imported without a
    # display server (useful in test environments).
    from PySide6.QtWidgets import QApplication

    app = QApplication(sys.argv)
    app.setApplicationName("Roota")
    app.setApplicationVersion("0.1.0")

    # TODO: instantiate MainWindow once UI module is implemented
    # from app.ui.window import MainWindow
    # window = MainWindow()
    # window.show()

    print("Roota initialised — UI not yet implemented.")
    sys.exit(0)


if __name__ == "__main__":
    main()
