// Loaded only when `npm run dev` is opened in a browser; never part of the production bundle.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type { AppEnvironment, DownloadItem, DownloadRequest, MediaInfo, VideoOption } from "../types";

const video = (height: number, label: string, shortLabel: string, mb: number): VideoOption => ({
  height, width: Math.round((height * 16) / 9), label, shortLabel, fps: 30,
  sizeMp4: mb * 1e6, sizeMkv: mb * 0.8e6, videoBytesMp4: mb * 0.9e6, videoBytesMkv: mb * 0.7e6,
});

const single: MediaInfo = {
  isPlaylist: false,
  url: "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
  platform: "YouTube",
  title: "Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film",
  uploader: "Blender",
  thumbnail: null,
  duration: 635,
  videoOptions: [
    video(2160, "4K", "4K", 1393), video(1440, "2K", "2K", 504), video(1080, "1080p Full HD", "1080p", 288),
    video(720, "720p HD", "720p", 181), video(480, "480p", "480p", 59), video(360, "360p", "360p", 29),
    video(240, "240p", "240p", 40), video(144, "144p", "144p", 35),
  ],
  audioOptions: [
    { kbps: 320, label: "320 kbps Studio", size: 25.4e6 }, { kbps: 256, label: "256 kbps High", size: 20.3e6 },
    { kbps: 192, label: "192 kbps Normal", size: 15.2e6 }, { kbps: 128, label: "128 kbps Small", size: 10.2e6 },
  ],
  wavSize: 122e6,
  audioSourceKbps: 160,
  audioBytes: 12e6,
  hasSubtitles: true,
  entries: [],
  parentPlaylistUrl: null,
};

const playlist: MediaInfo = {
  ...single,
  isPlaylist: true,
  platform: "YouTube",
  title: "Blender Open Movies",
  duration: 2400,
  entries: ["Big Buck Bunny", "Sintel", "Tears of Steel", "Elephants Dream", "Spring"].map((title, i) => ({
    id: String(i), url: `https://youtu.be/${i}`, title, duration: 300 + i * 120, thumbnail: null,
  })),
};

const tiktok: MediaInfo = {
  ...single,
  platform: "TikTok",
  title: "TikTok video #7137423965982592302",
  uploader: "charlidamelio",
  duration: 11,
  hasSubtitles: false,
  videoOptions: [video(1280, "720p HD", "720p", 1.4), video(1024, "576p", "576p", 2.8)],
};

let items: DownloadItem[] = [];

function push(item: DownloadItem) {
  items = items.map((i) => (i.id === item.id ? item : i));
  void emit("download:update", item);
}

function simulate(id: string) {
  const timer = window.setInterval(() => {
    const item = items.find((i) => i.id === id);
    if (!item || item.status !== "active") return window.clearInterval(timer);
    const percent = Math.min(item.percent + 4, 100);
    if (percent >= 100) {
      window.clearInterval(timer);
      push({ ...item, percent: 100, status: "completed", speedBps: null, etaSecs: null, filePath: `/Downloads/${item.title}.${item.tag.toLowerCase()}` });
    } else {
      push({ ...item, percent, speedBps: 7.4e6, etaSecs: Math.round((100 - percent) / 4), stage: percent > 90 ? "merging" : "downloading" });
    }
  }, 600);
}

export function installMocks() {
  mockWindows("main");
  const params = new URLSearchParams(window.location.search);
  if (params.has("seed")) {
    const base = { thumbnail: null, platform: "YouTube", stage: "downloading" as const, speedBps: null, etaSecs: null, error: null, filePath: null };
    items = [
      { ...base, id: "a", title: "Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film", tag: "MP4", quality: "1080p", status: "active", percent: 62, speedBps: 8.2e6, etaSecs: 42 },
      { ...base, id: "b", title: "Sintel - Tam film", tag: "MP3", quality: "320kbps", status: "active", stage: "converting", percent: 99 },
      { ...base, id: "c", platform: "TikTok", title: "TikTok video #7137423965982592302", tag: "MP4", quality: "720p", status: "completed", percent: 100, filePath: "/x.mp4" },
      { ...base, id: "d", platform: "Instagram", title: "Reel — sign-in required", tag: "MP4", quality: "1080p", status: "failed", percent: 0, error: "This content requires signing in. Pick a browser session below the link box and try again." },
      { ...base, id: "e", title: "Tears of Steel", tag: "MKV", quality: "4K", status: "queued", percent: 0 },
    ];
  }

  mockIPC(async (cmd, raw) => {
    const args = raw as Record<string, never>;
    switch (cmd) {
      case "fetch_metadata": {
        await new Promise((r) => setTimeout(r, 900));
        const url = String(args.url);
        if (url.includes("error")) throw "This link is not supported.";
        if (url.includes("playlist")) return playlist;
        if (url.includes("tiktok")) return tiktok;
        return single;
      }
      case "set_language":
        return null;
      case "start_download": {
        const request = args.request as unknown as DownloadRequest;
        const selection = request.job.selection;
        const item: DownloadItem = {
          id: crypto.randomUUID(), title: request.title, thumbnail: request.thumbnail, platform: request.platform,
          tag: (selection.mode === "video" ? selection.container : selection.format).toUpperCase(),
          quality: request.job.qualityLabel, status: "active", stage: "downloading", percent: 0,
          speedBps: null, etaSecs: null, error: null, filePath: null,
        };
        items = [...items, item];
        simulate(item.id);
        return item;
      }
      case "list_downloads":
        return items;
      case "cancel_download": {
        const item = items.find((i) => i.id === String(args.id));
        if (item) push({ ...item, status: "canceled", speedBps: null, etaSecs: null });
        return null;
      }
      case "retry_download": {
        const item = items.find((i) => i.id === String(args.id));
        if (item) {
          push({ ...item, status: "active", percent: 0, error: null });
          simulate(item.id);
        }
        return null;
      }
      case "remove_download":
        items = items.filter((i) => i.id !== String(args.id));
        void emit("download:removed", String(args.id));
        return null;
      case "app_environment":
        return { os: "macos", isMobile: false, engine: "preview", downloadDir: "~/Downloads", tools: [] } satisfies AppEnvironment;
      case "plugin:clipboard-manager|read_text":
        return "https://www.youtube.com/watch?v=aqz-KE-bpKQ";
      case "plugin:event|listen":
      case "plugin:event|unlisten":
      case "plugin:event|emit":
        return null;
      default:
        return null;
    }
  }, { shouldMockEvents: true });
}
