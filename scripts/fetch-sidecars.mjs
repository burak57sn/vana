#!/usr/bin/env node
// Downloads the desktop tools for the current target. ffmpeg, ffprobe and deno are named
// the way Tauri's `externalBin` expects: <name>-<triple>[.exe]. yt-dlp is the unpacked
// (onedir) release in binaries/yt-dlp-dist, bundled as a resource: the single-file build
// unpacks itself on every run, which costs over 10 s per call on macOS.
//
// A system-wide install is only reported, never copied: Homebrew's ffmpeg links
// against dylibs and its yt-dlp is a Python script, so neither survives bundling.
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "src-tauri", "binaries");
const ytDlpDir = join(outDir, "yt-dlp-dist");
const ytDlpMarker = join(outDir, ".yt-dlp-dist-target");

const YT = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const RIEDL = "https://ffmpeg.martin-riedl.de/redirect/latest";
const BTBN = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";
const DENO = "https://github.com/denoland/deno/releases/latest/download";

const riedl = (arch) => ({
  ffmpeg: { url: `${RIEDL}/macos/${arch}/release/ffmpeg.zip`, member: "ffmpeg" },
  ffprobe: { url: `${RIEDL}/macos/${arch}/release/ffprobe.zip`, member: "ffprobe" },
});
const btbn = (pkg, ext = ".exe") => ({
  ffmpeg: { url: `${BTBN}/${pkg}`, member: `ffmpeg${ext}` },
  ffprobe: { url: `${BTBN}/${pkg}`, member: `ffprobe${ext}` },
});
const deno = (triple, ext = "") => ({ deno: { url: `${DENO}/deno-${triple}.zip`, member: `deno${ext}` } });

const TARGETS = {
  "aarch64-apple-darwin": { ytDlp: "yt-dlp_macos.zip", ...riedl("arm64"), ...deno("aarch64-apple-darwin") },
  "x86_64-apple-darwin": { ytDlp: "yt-dlp_macos.zip", ...riedl("amd64"), ...deno("x86_64-apple-darwin") },
  "x86_64-pc-windows-msvc": { ytDlp: "yt-dlp_win.zip", ...btbn("ffmpeg-master-latest-win64-gpl.zip"), ...deno("x86_64-pc-windows-msvc", ".exe") },
  "aarch64-pc-windows-msvc": { ytDlp: "yt-dlp_win_arm64.zip", ...btbn("ffmpeg-master-latest-winarm64-gpl.zip"), ...deno("x86_64-pc-windows-msvc", ".exe") },
  "x86_64-unknown-linux-gnu": { ytDlp: "yt-dlp_linux.zip", ...btbn("ffmpeg-master-latest-linux64-gpl.tar.xz", ""), ...deno("x86_64-unknown-linux-gnu") },
};

function hostTriple() {
  if (process.env.TAURI_ENV_TARGET_TRIPLE) return process.env.TAURI_ENV_TARGET_TRIPLE;
  try {
    return execFileSync("rustc", ["--print", "host-tuple"], { encoding: "utf8" }).trim();
  } catch {
    const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
    if (process.platform === "darwin") return `${arch}-apple-darwin`;
    if (process.platform === "win32") return `${arch}-pc-windows-msvc`;
    return `${arch}-unknown-linux-gnu`;
  }
}

async function download(url, dest) {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) throw new Error(`${url} → HTTP ${res.status}`);
  const bytes = Buffer.from(await res.arrayBuffer());
  writeFileSync(dest, bytes);
  return bytes;
}

async function verifyYtDlp(url, bytes) {
  const name = url.split("/").pop();
  const res = await fetch(`${YT}/SHA2-256SUMS`, { redirect: "follow" });
  if (!res.ok) throw new Error(`SHA2-256SUMS → HTTP ${res.status}`);
  const line = (await res.text()).split("\n").find((l) => l.trim().endsWith(` ${name}`) || l.trim().endsWith(`*${name}`));
  if (!line) throw new Error(`no checksum found for ${name}`);
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== line.split(/\s+/)[0]) throw new Error(`checksum mismatch for ${name}`);
}

function findFile(dir, name) {
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) {
      const hit = findFile(p, name);
      if (hit) return hit;
    } else if (entry === name) return p;
  }
  return null;
}

function extract(archive, into) {
  if (archive.endsWith(".zip") && process.platform !== "win32") execFileSync("unzip", ["-q", "-o", archive, "-d", into]);
  else execFileSync("tar", ["-xf", archive, "-C", into]);
}

function reportSystemTool(name) {
  const res = spawnSync(process.platform === "win32" ? "where" : "which", [name], { encoding: "utf8" });
  return res.status === 0 ? res.stdout.split("\n")[0].trim() : null;
}

// A single folder serves every target, so a marker records which one it was unpacked for.
async function fetchYtDlp(triple, asset, ext, work, force) {
  const current = existsSync(ytDlpMarker) ? readFileSync(ytDlpMarker, "utf8").trim() : null;
  if (!force && current === triple && existsSync(join(ytDlpDir, `yt-dlp${ext}`))) return;

  console.log(`[vana] yt-dlp: downloading the folder build for ${triple}…`);
  const url = `${YT}/${asset}`;
  const file = join(work, asset);
  await verifyYtDlp(url, await download(url, file));
  rmSync(ytDlpDir, { recursive: true, force: true });
  mkdirSync(ytDlpDir, { recursive: true });
  extract(file, ytDlpDir);

  const launcher = readdirSync(ytDlpDir).find((entry) => entry.startsWith("yt-dlp"));
  if (!launcher) throw new Error(`yt-dlp not found in ${asset}`);
  renameSync(join(ytDlpDir, launcher), join(ytDlpDir, `yt-dlp${ext}`));
  if (!ext) chmodSync(join(ytDlpDir, `yt-dlp${ext}`), 0o755);
  writeFileSync(ytDlpMarker, triple);
  console.log(`[vana] yt-dlp: done → ${ytDlpDir.replace(root + "/", "")}`);
}

async function main() {
  // Android cannot execute desktop binaries; see src-tauri/src/engine/mobile.rs.
  if (process.env.TAURI_ENV_PLATFORM === "android" || process.env.TAURI_ENV_PLATFORM === "ios") return;

  const triple = hostTriple();
  const spec = TARGETS[triple];
  if (!spec) throw new Error(`Unsupported target: ${triple}`);
  const ext = triple.includes("windows") ? ".exe" : "";
  mkdirSync(outDir, { recursive: true });
  const force = process.argv.includes("--force");
  const work = mkdtempSync(join(tmpdir(), "vana-sidecars-"));

  try {
    await fetchYtDlp(triple, spec.ytDlp, ext, work, force);

    const archives = new Map();
    for (const [name, source] of Object.entries(spec)) {
      if (name === "ytDlp") continue;
      const target = join(outDir, `${name}-${triple}${ext}`);
      if (existsSync(target) && !force) continue;

      const system = reportSystemTool(name);
      console.log(`[vana] ${name}: ${system ? `found on the system (${system}), downloading a portable build` : "downloading"}…`);

      let dir = archives.get(source.url);
      if (!dir) {
        dir = mkdtempSync(join(work, "x-"));
        const file = join(work, `${name}-${source.url.split("/").pop()}`);
        await download(source.url, file);
        extract(file, dir);
        archives.set(source.url, dir);
      }
      const found = findFile(dir, source.member);
      if (!found) throw new Error(`${source.member} not found in ${source.url}`);
      copyFileSync(found, target);
      if (!ext) chmodSync(target, 0o755);
      console.log(`[vana] ${name}: done → ${target.replace(root + "/", "")}`);
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

main().catch((err) => {
  console.error(`[vana] sidecar setup failed: ${err.message}`);
  process.exit(1);
});
