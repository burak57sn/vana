# Third-party notices

Vana's own source code is under the MIT License (see `LICENSE`). The installers also
ship the following programs unmodified, as separate executables. Each keeps its own license.

| Program | License | Source |
|---|---|---|
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | The Unlicense (public domain); its bundled Python runtime and libraries keep their own licenses | https://github.com/yt-dlp/yt-dlp |
| [FFmpeg](https://ffmpeg.org) (`ffmpeg`, `ffprobe`) | GNU GPL v3 (built with `--enable-gpl`, includes x264) | https://ffmpeg.org/download.html |
| [Deno](https://deno.com) | MIT | https://github.com/denoland/deno |

FFmpeg builds come from [Martin Riedl](https://ffmpeg.martin-riedl.de) (macOS) and
[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) (Windows). The complete
corresponding source code of FFmpeg is available from those pages and from
https://ffmpeg.org. The GPL applies to those FFmpeg binaries, not to Vana's own code,
which only runs them as separate programs.

Rust and npm dependencies are listed in `src-tauri/Cargo.lock` and `package-lock.json`;
each keeps its own license.
