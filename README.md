# Vana

Paste a link, open the valve, let Vana handle the rest.

Vana is a desktop downloader for **publicly available** video and audio from YouTube, TikTok, Instagram Reels and X, built with Tauri v2, Rust, React and Tailwind CSS. The interface is in English and Turkish (switch with **EN / TR** in the top corner).

## Features

- **Video:** MP4 or MKV, from 144p up to 8K, with optional embedded subtitles. Streams are copied, never re-encoded.
- **Audio:** MP3, M4A or WAV, with cover art and tags.
- **Playlists:** pick songs or videos with checkboxes and queue them all at once.
- **Queue:** up to 3 downloads in parallel, with progress, speed, cancel and retry.
- **Tray:** the valve icon in the menu bar or system tray turns as downloads progress. Closing the window keeps downloads running.

Everything Vana needs (yt-dlp, FFmpeg, Deno) ships inside the installer. Nothing else to install.

## Download

Get the latest installer from [Releases](https://github.com/burak57sn/vana/releases).

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `Vana_x.y.z_aarch64.dmg` |
| Windows 10/11 (x64) | `Vana_x.y.z_x64-setup.exe` |

The builds are not signed with a paid certificate yet, so the system warns on first launch:

- **macOS:** open the app once, then go to **System Settings → Privacy & Security** and click **Open Anyway**.
- **Windows:** in the SmartScreen dialog click **More info → Run anyway**. If WebView2 is missing, the installer downloads it.

Android is experimental and not released: the interface runs, but there is no download engine on Android yet.

## Legal notice

Vana is a tool for downloading content **you have the right to download**: your own uploads, content under a permissive license (for example Creative Commons) or content whose owner allows downloading. Downloading copyrighted material without permission may break the law where you live and the terms of service of the platform it comes from. You are responsible for how you use Vana.

Vana only works with publicly accessible content. It does not log in to accounts or read browser sessions, does not bypass DRM or paywalls, and does not remove watermarks. Vana is not affiliated with or endorsed by YouTube, TikTok, Instagram, X or any other platform. All trademarks belong to their owners.

## Build from source

Requirements: Node.js 22+, Rust (stable), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev        # development; downloads the sidecars when needed
npm run tauri build      # installer for the current OS
npm run check            # tsc and cargo check
```

`scripts/fetch-sidecars.mjs` downloads yt-dlp, FFmpeg, FFprobe and Deno for the target into `src-tauri/binaries/` and checks yt-dlp's SHA-256. These binaries are not in the repository.

Pushing to `main` builds the macOS and Windows installers with GitHub Actions (`.github/workflows/release.yml`). Pushing a `v*` tag also attaches them to a draft release.

**Windows from macOS (optional):** needs `cargo-xwin`, LLVM and NSIS (`brew install llvm nsis`, `cargo install --locked cargo-xwin`):

```bash
LC_ALL=en_US.UTF-8 npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

This replaces `src-tauri/binaries/yt-dlp-dist` with the Windows build. The next macOS build restores it automatically.

## Speed

- yt-dlp ships as its unpacked folder build. The single-file build unpacks itself on every run, which costs 10+ seconds per call.
- `--concurrent-fragments 16` speeds up fragmented streams (HLS, DASH).
- If `aria2c` is installed (in `PATH`, `/opt/homebrew/bin` or `/usr/local/bin`), Vana uses it with 16 connections. It is not bundled.

## Tests

```bash
cd src-tauri
cargo test --lib
cargo build && cargo test --test e2e -- --ignored --nocapture --test-threads=1
```

The `e2e` tests use the real network. Set `VANA_BIN_DIR=/path/to/Vana.app/Contents/MacOS` to run them against the tools inside a packaged app.

Opening `npm run dev` in a plain browser runs the UI on sample data from `src/dev/mocks.ts` (`?seed` fills the queue).

## Architecture

| Path | Role |
|---|---|
| `src-tauri/src/engine/mod.rs` | `DownloaderEngine` trait |
| `src-tauri/src/engine/desktop.rs` | macOS/Windows engine that runs the bundled tools |
| `src-tauri/src/engine/args.rs`, `metadata.rs`, `progress.rs` | Platform-independent logic with unit tests |
| `src-tauri/src/engine/mobile.rs` | Android skeleton where a native engine can plug in |
| `src-tauri/src/queue.rs` | Download queue and UI events |
| `src-tauri/src/tray.rs` | Menu bar / tray icon that turns with progress |
| `src-tauri/src/i18n.rs`, `src/i18n.tsx` | English and Turkish texts |
| `scripts/generate-icons.py` | App and tray icons |

## License

[MIT](LICENSE) © 2026 burak57sn. The bundled yt-dlp, FFmpeg and Deno keep their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
