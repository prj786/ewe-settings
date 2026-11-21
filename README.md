# ewe-settings

**ewe Settings**, the settings app for [ewe](https://github.com/prj786/ewe) —
an Arch-only Hyprland desktop. Tauri v2 + Svelte 5. The display name is
"ewe Settings"; the package, the binary, the desktop file and the window
class stay `ewe-settings`. Formerly `hypr-shell-settings`; the
package `provides`/`replaces` the old name and ships a `hypr-settings`
compatibility symlink.

## Why it is a separate process

The shell itself is Quickshell/QML, and everything that has to be a Wayland
layer-shell surface — the bar, dock, notifications, OSD, lock screen — stays
there, because Tauri cannot create layer-shell surfaces at all.

Settings is the one piece that does not need to be a layer surface: it is an
ordinary window. Moving it out keeps a large, rarely-open UI out of the shell
process, where a QML error takes the whole desktop down with it.

## How it talks to the shell

It does not. It writes the same files the shell already reads, then asks the
shell to re-read them:

```
Settings  ──writes──>  ~/.config/quickshell/user-theme.json
          ──pokes───>  qs ipc call settings reload
```

That is the contract, and it has three useful properties: there is still one
source of truth, a change survives a shell restart for free, and the existing
Google Drive sync needs no changes — those files are exactly what it already
backs up.

Writes are atomic (temp file + rename) and **merge** rather than replace, since
the shell writes this file too. The shell side merges for the same reason.

The IPC verbs this app depends on (`reload`, `ping`, `version`) live in
ewe's `Settings.qml`. They are public API in both directions: renaming
one breaks an installed binary.

If the shell is not running, writes still succeed — they simply take effect at
the next login. Nothing here requires the shell to be up.

## Privileges

No helper and no setuid anything: the files it writes are the user's own.
The few system changes go through `pkexec` — one polkit prompt each, fixed
scripts, every value checked first: the time zone latch, the display language
and the regional formats (`localectl set-locale`, generating the locale when
needed, always naming LANG and the formats together so one never drops the
other), and on-demand installs. `install_packages` (the one install path, also
for input methods) takes official-repo packages of a few families only —
`hunspell-*`, `firefox-i18n-*`, `thunderbird-i18n-*`, `libreoffice-fresh-*` /
`-still-*`, `fcitx5*` — checks them against the sync databases, skips what is
installed and runs `pacman -S --needed --noconfirm`. A dictionary only the AUR
has opens Komble's search (`komble --search=hunspell-ka`) instead.

## Language (Settings → Language, Time and place → Formats)

- Spelling languages are `[desktop.language] spellcheck` in ewe.conf (a list
  of hunspell names, written through `ewe-conf set`; unset = the display
  language's dictionary when installed + `en_US`) and `check_spelling` (bool,
  default true). They sync, so a name with no dictionary here shows "not on
  this computer" with Download / Open Komble. ewe Browser reads both and
  follows changes; it checks with Chromium's own dictionaries, so the page
  marks a language Chromium lacks (Georgian…) as "Not in ewe Browser yet".
- Formats = LC_TIME, LC_NUMERIC, LC_MONETARY, LC_MEASUREMENT, LC_PAPER from
  one region, separate from LANG; "Same as the display language" clears them.
  Apps read them at start: the page says "Applies after you sign out".
- Also for your apps: Firefox / Thunderbird / LibreOffice language packs for
  the display language, offered only for the apps that are installed.

## Version

The footer shows **ewe's** version, not this app's. Settings is part of
the desktop rather than a product with its own release cycle, and a second
number would only raise the question of which one is real. It is read from the
repo checkout, falling back to what the running shell reports.

The checkout is found via `$EWE_REPO` (or the legacy `$HYPR_SHELL_REPO`), then
`~/.local/share/ewe` (a get.sh install), then `~/hypr-shell` (a developer clone).

## Build

```bash
npm install
npm run tauri dev
npm run check   # Find a setting's index still matches the pages
```

`dev-mock.html` (on the dev server) fakes the Tauri bridge so every page
can be looked at in a browser; its header lists the query parameters.

Packaging is a `PKGBUILD` — Arch only, built on Arch:

```bash
makepkg -si
```

Two things in that PKGBUILD are load-bearing and were each a real bug:

- `options=(!lto)` — LTO breaks linking against the system webkit stack.
- `npm run tauri build -- --no-bundle`, never a bare `cargo build --release`.
  `tauri-build` decides dev-vs-production at compile time; a plain cargo build
  stays in dev mode and bakes `devUrl` (localhost) into the binary, so the
  installed app opens to "Could not connect to localhost".

## Licence

MIT. See [LICENSE](LICENSE).
