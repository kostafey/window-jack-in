[![Unlicense Public domain](https://img.shields.io/badge/license-Public_domain-green.svg)](https://wiki.creativecommons.org/wiki/Public_domain)

# window-jack-in

<img src="https://raw.githubusercontent.com/kostafey/window-jack-in/master/src/window-jack-in.png" width="64px"
 alt="window-jack-in logo" align="right" />
 
A small Windows tray program that binds a **global hotkey per application**
and brings that application's window to the foreground immediately — press
again to cycle through its other windows.

```
+--------------------+      +--------------------+
|                    |      |.....target app....x|
|                    |      |.                  .|
|      O     O       |      |.                  .|
|         |          |  =>  |.                  .|
|      \_____/       |      |.                  .|
|                    |      |.                  .|
|                    |      |....................|
+--------------------+      +--------------------+
          ||                          ||
       --------                    --------
```

## What it does

- Binds one **global hotkey per target application** — Chrome, Edge, Slack,
  muCommander, Windows Terminal, … — each with its own hotkey, set in the
  config.
- Uses `RegisterHotKey`, so the combination is consumed by the system (no
  leaked keystrokes into the active app) and the tray process is permitted to
  change the foreground window.
- Finds and activates the target window itself, cycling through multiple
  matches on repeat press.
- Supports per-target actions beyond activation (e.g. minimize the foreground
  window, switch keyboard layout).

## Configuration

Configuration lives in `%APPDATA%\window-jack-in\config.toml`, materialised
with defaults on first run. Each target entry matches a window (by executable
and/or title) and assigns a hotkey and an action.

## Usage

```
window-jack-in.exe --list-windows   # dump HWND / class / exe / title table
window-jack-in.exe --check-config   # validate hotkeys + launch paths
window-jack-in.exe --config PATH    # use a specific config file
```

Run it with no arguments to start the tray program.

## Build

```
cargo build --release
```

Note on elevation: if the target window belongs to an elevated process and
the tray program does not, Windows UIPI blocks foreground changes and
`RegisterHotKey` won't fire over an elevated foreground. Run the tray elevated
only if you need to control elevated apps.

## Signing the build

Windows is wary of an unsigned tray-and-hotkey binary and may refuse to run a
freshly built `window-jack-in.exe`. `tools/sign.ps1` Authenticode-signs the
exe with a locally generated code-signing certificate so the system trusts
your own build. Run it from an **elevated** PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File tools\sign.ps1
```

On first run it creates a self-signed code-signing certificate
(`CN=window-jack-in local dev`) in your user store and installs its public
part into the machine `Root` and `TrustedPublisher` stores — the step that
needs elevation. Later runs reuse that certificate. It then signs the exe
with SHA256 and a timestamp, and prints the signature status and the file's
SHA256.

Options:

```powershell
# sign a binary at a non-default path
powershell -ExecutionPolicy Bypass -File tools\sign.ps1 -ExePath other\window-jack-in.exe

# use your own certificate subject — reuse the SAME subject on every run, or a
# second certificate gets created instead of the first being reused
powershell -ExecutionPolicy Bypass -File tools\sign.ps1 -Subject "CN=window-jack-in local dev, O=your-name"
```

The signature is bound to the exact bytes of the exe, so **re-run `sign.ps1`
after every `cargo build`**. Rebuilding also needs the running tray closed
first (Windows locks the exe while the process is alive) — quit it from the
tray menu, or `Get-Process window-jack-in | Stop-Process -Force`.
