# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

This repository is a **Windows-only Rust program** that focuses/raises
application windows from global hotkeys: it binds one global hotkey per
configured target application and brings that app's window to the foreground.
See `README.md` for the user-facing description.

## What this binary does

`window-jack-in.exe` is a tray-resident helper that binds one global hotkey per
configured target application. On hotkey press it enumerates top-level windows,
picks the one matching the target's predicates (class / exe / title), and
brings it to the foreground — cycling through hits on repeat press. If no
match exists (or `always_launch_if_no_window = true`), a configured
`launch` command is spawned detached with no inherited handles.

Targets, hotkeys, matchers, and launch commands are all read from a TOML
config.

## Module layout

Single binary. `build.rs` embeds the application icon as a Win32 resource (see
the `window-jack-in.ico` entry below). Files under `src/`:

- `main.rs` — `#![windows_subsystem = "windows"]` entry, arg parsing, mode
  dispatch (tray run / `--list-windows` / `--check-config`), singleton mutex,
  no panics — all failures go through `util::message_box_error`.
- `config.rs` — TOML schema (`RawConfig`/`RawTarget`/`RawMatcher`), hotkey
  string parser (`Ctrl+Alt+E` → `MOD_* | MOD_NOREPEAT`, VK code), title
  pattern matcher (`(?i)` prefix + `|` alternation of case-insensitive
  substrings — deliberately *not* a full regex, keeps the binary small),
  config search order (`--config` → `%APPDATA%\window-jack-in\config.toml` →
  `<exe dir>\window-jack-in.toml` → embedded default).
- `default_config.toml` — embedded via `include_str!`. Materialised into
  `%APPDATA%` on first run.
- `window.rs` — `EnumWindows` filtering (`IsWindowVisible`, `GW_OWNER`,
  `WS_EX_TOOLWINDOW`, `DwmGetWindowAttribute(DWMWA_CLOAKED)`), exe path via
  `QueryFullProcessImageNameW` + `PROCESS_QUERY_LIMITED_INFORMATION`,
  activation with the `AttachThreadInput` dance and
  `AllowSetForegroundWindow(ASFW_ANY)` before spawns, HWND-sorted cyclic
  switching.
- `tray.rs` — the hidden receiver window (**not** `HWND_MESSAGE` — that
  variant doesn't receive `WM_TASKBARCREATED`), `Shell_NotifyIconW`
  install/refresh, `RegisterHotKey`/`WM_HOTKEY`, popup menu (targets +
  Reload/List/Quit), single message loop with no worker threads.
- `util.rs` — wide-string helpers, `MessageBoxW` error/warning reporters,
  named-mutex singleton (`Global\window-jack-in-singleton`), `%VAR%`/`~`
  expansion via `ExpandEnvironmentStringsW`, App Paths registry lookup,
  detached `CreateProcessW` with `CREATE_NO_WINDOW | DETACHED_PROCESS` and
  no inherited handles, `AttachConsole(ATTACH_PARENT_PROCESS)` for
  diagnostic modes, opt-in file logger.
- `window-jack-in.ico` — embedded as a Win32 resource (icon group, id `1`) by
  `build.rs` via the `winresource` crate. Explorer, the desktop, the taskbar
  and Alt-Tab use it as the application icon; `tray::load_embedded_icon` loads
  the same resource at the small-icon size with `LoadImageW`, so the icon
  ships once (no `include_bytes!` copy, no hand-rolled ICO parser). The ico is
  capped at 128 px to keep the binary small. Building therefore needs a
  resource compiler: `windres` (mingw-w64 binutils) on the GNU target, or
  `rc.exe` / `llvm-rc` on MSVC. The rustup GNU toolchain does not bundle
  `windres`, so install mingw-w64 binutils and put it on PATH.

## Build

```powershell
cargo build --release                                 # msvc if default
cargo build --release --target x86_64-pc-windows-gnu  # explicit gnu — the target the task pins
```

Release profile is tuned for minimum size (`opt-level = "z"`, `lto = true`,
`codegen-units = 1`, `panic = "abort"`, `strip = true`). The current release
binary is ~484 KB — the task's acceptance criterion is < 500 KB, so watch
this budget when adding dependencies. `regex-lite` was removed for this
reason; use `TitlePattern` instead.

No test suite — `cargo check` / `cargo clippy` are the feedback loop.

## Running it

```powershell
window-jack-in.exe                  # run tray
window-jack-in.exe --help
window-jack-in.exe --list-windows   # dump HWND/class/exe/title table
window-jack-in.exe --check-config   # parse + validate hotkeys + launches
window-jack-in.exe --config PATH    # use specific TOML
window-jack-in.exe --log PATH       # append diagnostics to file
```

Diagnostic modes call `AttachConsole(ATTACH_PARENT_PROCESS)` so output
appears in the shell that launched them, despite `windows_subsystem =
"windows"`.

## Things to know before editing

- **Do not add worker threads for hotkey handling.** The whole point of using
  `RegisterHotKey` + `WM_HOTKEY` in the main message loop (rather than
  `inputbot`'s `WH_KEYBOARD_LL` hook) is that (a) the system consumes the
  key so the letter doesn't leak into the active app, and (b) `WM_HOTKEY`
  gives the message-loop thread the right to change the foreground window.
  A background thread doesn't have that right and would need
  `AllowSetForegroundWindow` gymnastics.
- **`spawn_detached` uses `CreateProcessW` directly**, not
  `std::process::Command`. This is on purpose: the old code hit
  `Command::output()` which blocks on stdout/stderr pipe EOF, and the
  `--alternate-editor=runemacs.exe` fallback inherits those handles and
  never closes them until Emacs quits. Any new spawn path must set
  `bInheritHandles = FALSE` and `DETACHED_PROCESS | CREATE_NO_WINDOW`.
- **Anti-virus false positives are expected.** `RegisterHotKey` +
  `Shell_NotifyIconW` + unsigned binary is a common shape for benign
  keyloggers; Defender / corporate EDR will sometimes block execution
  right after `cargo build`. When smoke-testing, either sign the exe or
  add a Defender exclusion for the release dir.
- **Windows Elevation (UIPI):** if the target app is elevated and the tray
  is not, `SetForegroundWindow` and even the `RegisterHotKey` fire will
  fail against the elevated foreground. This is an intentional non-goal —
  elevate the tray only if you need it to control elevated apps.

## EDR / execution policy

Corporate EDR (CrowdStrike Falcon in the observed setup, but the same shape
applies to Defender ML at high sensitivity) **silently blocks the freshly-
built exe** with `Access is denied` from PowerShell — no popup, no toast.
Diagnostic markers seen when it happens:

- `Test-Path` on the exe is `True` (not quarantined).
- `Get-WinEvent -LogName System` under provider `csagent` shows nothing.
- `Get-WinEvent -LogName 'Microsoft-Windows-Windows Defender/Operational'`
  has no detection events for the exe.
- `Get-AppLockerPolicy -Effective -Xml` reports AppLocker present but not
  enforcing → not an AppLocker block.
- `Get-WinEvent -LogName 'Microsoft-Windows-CodeIntegrity/Operational'`
  around the launch time shows event 3004 for
  `\Windows\System32\ScriptControl64_<version>.dll` (Falcon's own script-
  control DLL injected on process creation) — **the exe launch is being
  observed by Falcon**, but the actual block decision is not logged to a
  Windows Event Log (it goes to the Falcon Cloud Console via a kernel ETW
  channel only IT/security can read).

The remedy that has worked without an IT ticket: **self-sign the exe with a
locally-generated code-signing cert whose public part is added to
`LocalMachine\Root` and `LocalMachine\TrustedPublisher`.** `tools/sign.ps1`
does this idempotently:

```powershell
# From an elevated PowerShell. -Subject is a parameter; substitute your own
# name/org. Use the SAME -Subject on every run — the script looks the cert
# up by subject in Cert:\CurrentUser\My, so a different string creates a
# second cert instead of reusing the first.
powershell -ExecutionPolicy Bypass -File tools\sign.ps1 `
    -Subject "CN=window-jack-in local dev, O=firstname.lastname"
```

After signing, retry `.\target\release\window-jack-in.exe --help`. If it
still blocks, only remaining path is an IT ticket with the SHA256 the script
prints at the end.

**Signature is bound to the exact exe bytes** — every `cargo build`
invalidates it, so `tools\sign.ps1` must be re-run after each rebuild.
Before rebuilding, kill the running tray (`Get-Process window-jack-in |
Stop-Process -Force`, or `Quit` from the tray menu) — Windows locks the
exe file while the process is alive and `cargo build` fails with
`Access is denied` otherwise.

Two smaller changes are also in place unconditionally, because they cheaply
reduce heuristic score:

- Singleton mutex is `Local\...`, not `Global\...` (`src/main.rs`).
- `CreateProcessW` is called with `dwCreationFlags = 0`, **not**
  `DETACHED_PROCESS | CREATE_NO_WINDOW` (`src/util.rs::spawn_detached`).
  Handle-inheritance is still off, which is what actually prevents the
  original `output()`-style hang.

## `TitlePattern` is deliberately not a regex

`config::TitlePattern` supports only `(?i)` prefix + `|`-separated
case-insensitive substrings. This is enough for every entry in the default
config and lets us drop `regex-lite` (~40 KB). If a user configures
something like `title = "^Some.*$"`, the `^`/`.`/`*` are matched literally.
That's a documented limitation — do not silently promote this to a full
regex without measuring the binary-size impact.
