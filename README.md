<p align="center"><img src="docs/brand/sofa/assets/logo-horizontal.svg" alt="Sofá" height="72"></p>

**English** | [Português (Brasil)](README.pt-BR.md)

# Sofá

Sofá is a game launcher for Windows, made to be used on a TV with an Xbox controller. It shows your Steam, Epic Games and GOG games, plus any `.exe` you add, in one library. You open a game, play, and come back to the launcher without a keyboard or mouse.

> Status: pre-release `v0.1.0`. The installer is not code-signed (see [Download and install](#download-and-install)).

## Requirements

- Windows 10 or 11, 64-bit.
- An Xbox controller (wired or wireless). The keyboard works only as an emergency fallback.
- WebView2 runtime. If it is missing (more common on Windows 10), the installer downloads it from Microsoft, so that PC needs internet during installation.

## Download and install

1. Download `Sofa_0.1.0_x64-setup.exe` and `SHA256SUMS.txt` from the [v0.1.0 release](https://github.com/dev-azevedo/game-launcher-home-tv/releases/tag/v0.1.0). The release also has `README.txt`, `BUILD-INFO.txt` and `coletar-logs.ps1`.
2. Check the hash in PowerShell, in the folder of the file:

   ```powershell
   Get-FileHash .\Sofa_0.1.0_x64-setup.exe -Algorithm SHA256
   ```

   The value must match the one in `SHA256SUMS.txt`. If it differs, do not install.
3. Run the installer. It is **not signed**, so Windows SmartScreen may warn you. Continue only after the hash matched.
4. The installer asks for administrator permission (UAC) and installs for the whole PC.
5. The folder is fixed: `C:\Program Files\Sofa`. There is no folder page, and the `/D=` option with another folder is refused. An older installation outside Program Files is also refused: uninstall it first.
6. The "Start with Windows" page comes checked. Each Windows user can change it later in Settings.

To play without a keyboard right after turning the PC on, set up automatic sign-in or Windows Hello in Windows. Sofá starts on the desktop, and the installer does not change your sign-in.

## Using it with the controller

| Button | What it does |
|---|---|
| D-pad | Move focus, change tab, adjust volume in the quick menu |
| A | Play, choose, confirm, go back to the running game |
| B | Back, cancel, close |
| View | Game options (in the library) |
| Menu | Quick menu |
| LB / RB | Jump rows |
| LT / RT | Volume down / up (in the library) |
| View + Menu | Return combination: back to Sofá while a game is running |

**Adding games.** Installed Steam, Epic and GOG games are detected on their own. To add something else, go to Settings, Library, Add game, and pick a `.exe` on a local disk. Network paths, command-line arguments and files that are not `.exe` are not accepted. Each card shows its origin (Steam, Epic, GOG or Manual). "Update library" looks for new games.

**Playing.** Move to a card and press A. While the game runs, its card shows "In progress" and A returns to it. When the game closes, Sofá comes back to the front.

**Back to Sofá during a game.** Press View + Menu together. A panel offers: back to the game, close game, go to the library (the game stays open) and quick menu. "Close game" asks the game to close; if it does not answer, "Force close" appears (unsaved progress is lost).

**Quick menu (Menu).** Volume, Mute, Shut down, Restart, Suspend, Settings and Exit launcher. Shut down, Restart and Suspend ask for confirmation, with Cancel in focus. The Windows shutdown is the normal one: if another app has unsaved work, Windows warns. Suspend shows as unavailable on PCs that do not support it.

**Volume.** LT and RT in the library, or the D-pad in the quick menu.

**Game options (View on a card).** Play (or Back to game), Rename, Restore original name, Hide. Renaming uses the PC keyboard. Hiding removes the game from the library and from updates and deletes no file. Hidden items come back in Settings, Hidden items.

**Settings.** Tabs: Display (which monitor Sofá opens on), Library (summary, update, add game), General (Start with Windows) and Hidden items.

**Start with Windows.** Opens Sofá when you sign in. It applies to your Windows user only.

### Emergency keyboard

| Key | Controller equivalent |
|---|---|
| Arrow keys | D-pad |
| Enter | A |
| Esc or Backspace | B |
| F1 or Menu key (ContextMenu) | Menu: opens the quick menu |
| F2 | View: game options in the library |
| PgUp / PgDn | LB / RB |
| `[` / `]` | LT / RT: volume down / up (library only) |

Inside a text field only Enter, Esc and Up/Down leave the field. There is no key for the return combination.

## Data and logs

Sofá keeps your library, hidden items, custom names, last-played dates and settings in `%LOCALAPPDATA%\app.sofa.launcher`. Logs are in `%LOCALAPPDATA%\app.sofa.launcher\logs`. Nothing leaves the PC on its own.

**Reporting a problem.** Download `coletar-logs.ps1` from the release and run it:

```powershell
powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1
```

It reads the data folder (and never changes it) and creates a zip on your desktop with the logs, `settings.json`, Windows, graphics and WebView2 information, and where Sofá is installed. It leaves out `library.json`, covers, your user name and your computer name. Send that zip with your report. Use `-IncluirBiblioteca` only if asked: it adds the list of games.

## Uninstall

Windows Settings, Apps, Installed apps, Sofá. The uninstaller asks whether to keep data, and **Keep data (library and settings)** comes checked. Checked, a later reinstall finds your library again. Unchecked, the data of your user is deleted. Updating over an existing install does not delete data.

## Common problems

- **SmartScreen warns about the installer.** Expected: it is not signed. Check the SHA-256 and continue only if it matches.
- **The installer fails on a PC without WebView2.** It downloads the runtime from Microsoft: connect the PC to the internet and try again. There is no offline installer.
- **The controller does not respond.** Sofá shows "No controller connected". Turn the controller on or plug in the cable. The keyboard works meanwhile.
- **The game does not come back to the front.** Open the game panel with View + Menu and choose "Back to game". If Windows did not let Sofá take the front, the game was minimized and "Back to game" restores it. A game that runs as administrator may ignore the return combination; use the keyboard (F1) or close the game from its own menu.
- **The installer refuses to continue.** Remove `/D=` from the command line, or uninstall the earlier copy that lives outside Program Files.

## For developers

Tauri 2 + Rust, with a React UI. Feature documentation is in `docs/` (spec, architecture, ADRs, security, API contract in `docs/api/launcher-tv.md`).

| Folder | Content |
|---|---|
| `crates/launcher-core` | Domain and application. No Tauri, no Windows, no I/O |
| `crates/launcher-win` | Windows adapters (persistence, logging, hardening, process probe, Job Object, controller, volume, power) |
| `src-tauri` | Presentation and composition root: commands, DTOs, capability, window, NSIS installer files in `windows/` |
| `ui` | Front end (Vite + React + TypeScript) |
| `scripts` | `ci.ps1`, guards, release and installer test scripts |

**Local CI.** There is no CI server. `scripts/ci.ps1` runs everything that must pass before a commit or a release:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1               # everything
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SkipAudit    # offline (record the reason)
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SkipUi       # back end only
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SelfTestOnly # guard self-test only
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -Release      # CI + MSVC release (installer and SHA256SUMS)
```

Steps: guard self-test, grep guards (architecture and security rules), `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, up-to-date TypeScript bindings (`ui/src/shared/ipc/generated`, from ts-rs; do not edit by hand), `cargo deny check`, `npm audit` and `npm run check` in `ui/`.

**Release.** `-Release` builds with the MSVC toolchain at the version pinned in `rust-toolchain.toml` and writes to `..\sofa-releases\v<version>` (`-ReleasesDir` changes it). It runs the MSVC tests (`-FullMsvcTests` for the whole workspace), `cargo deny` and `npm audit` reports, `tauri build` (NSIS, per-machine, embedded WebView2 bootstrapper), PE checks on `sofa.exe` (a single `RT_MANIFEST`, Control Flow Guard, ASLR, DEP), a Microsoft Defender scan of the installer and `sofa.exe` (the build fails on any detection), NSIS guards on the generated script, `scripts/test-installer.ps1` (installer and uninstaller tests that install nothing) and `SHA256SUMS.txt`. An official release comes from a `v<version>` tag on a clean tree; `-AllowUntagged` is for test builds.

**Toolchain.** Day-to-day development uses `x86_64-pc-windows-gnu`. Only the MSVC build is distributed (static WebView2 loader, Control Flow Guard, a single manifest). Do not put MinGW first in `PATH` when building for MSVC. `.cargo/config.toml` turns on `-C control-flow-guard` for the MSVC target.

The full technical notes, in Portuguese (guard list, installer tests, release steps), are in the developer section of [README.pt-BR.md](README.pt-BR.md#para-desenvolvedores).

## License

To be defined. There is no `LICENSE` file in this repository yet. The Sora and Figtree fonts keep their own OFL licenses (`docs/brand/sofa/assets/fonts`).
