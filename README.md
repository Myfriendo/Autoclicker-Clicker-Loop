# Pulse

A compact dark Windows mouse autoclicker, written in Rust with an egui interface. Input borders use a shade slightly darker than the boxes; the header contains only the Pulse name.

The Pulse icon is embedded in the executable and app window. `assets/pulse-icon.png` is the source artwork; `build.rs` converts it into a Windows icon resource during compilation. The distributed executable needs no separate image files.

## Run

Click **Hotkeys** beside the Pulse title to customize Start/stop, Emergency stop, and Capture position. Choose F1–F12, letters, or digits with optional Ctrl/Alt/Shift modifiers. Capture also supports left-click with at least one modifier. Duplicate shortcuts are rejected. Save applies changes and stores them in `%LOCALAPPDATA%\Pulse\hotkeys.txt`; Cancel discards edits and Reset defaults restores the original shortcuts in the editor. Hotkey editing is available when stopped; global shortcuts are suspended while the settings panel is open.

Double-click **Launch Pulse.bat** or run `target\release\pulse.exe`. The executable does not require Python or Rust to run.

To build from source, install Rust with the Windows MSVC toolchain and Visual Studio C++ build tools, then run `cargo build --release`. The launcher builds automatically if the executable is missing. Run `cargo test` for timing and session checks.

Select Left, Right, or Middle mouse button. Set the click interval using separate Days, Hours, Min, Sec, and Ms fields; blank fields count as zero and the units add together. For example, Sec = 1 and Ms = 500 means a 1.5-second interval. Fields accept non-negative whole numbers, with a minimum total click interval of 1 ms (default: 100 ms).

Choose **Run until stopped** for an unlimited session, or **Run for a duration** and enter the session length using the same separate time fields. Examples: Min = 1, Sec = 10, Hours = 1, or Days = 1. Timed sessions must have a positive duration and begin after the three-second startup countdown. Pauses count toward the session duration. Duration fields are disabled in unlimited mode.

Choose Current pointer to follow your mouse, or hold Ctrl and left-click a target to capture its screen coordinates (including on additional monitors). The capture click also reaches the target application: choose a harmless place when capturing.

Press F6 to start/stop from any application. Starting gives you three seconds to switch windows. F8 is the global emergency stop. Windows may reserve these shortcuts in some apps. This app pauses while its own window is foreground or a mouse button/modifier is held. Settings are locked during a run; capturing a new position stops the run. Closing Pulse stops all repetition.

Intervals are at least 1 ms and missed intervals are skipped. Timing is best effort and depends on Windows scheduling. There is no double-click mode, randomized human simulation, or ban-evasion logic. Automation must be permitted by the target app; pacing cannot guarantee compliance or prevent bans. Windows may block input into apps running with higher privileges.

## Acknowledgments

Developed with assistance from OpenAI Codex.

