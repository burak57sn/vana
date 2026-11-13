import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import type { AppEnvironment, DownloadItem, DownloadRequest, MediaInfo } from "./types";

export const api = {
  fetchMetadata: (url: string) => invoke<MediaInfo>("fetch_metadata", { url }),
  startDownload: (request: DownloadRequest) => invoke<DownloadItem>("start_download", { request }),
  listDownloads: () => invoke<DownloadItem[]>("list_downloads"),
  cancel: (id: string) => invoke<void>("cancel_download", { id }),
  retry: (id: string) => invoke<void>("retry_download", { id }),
  remove: (id: string) => invoke<void>("remove_download", { id }),
  open: (id: string) => invoke<void>("open_download", { id }),
  reveal: (id: string) => invoke<void>("reveal_download", { id }),
  environment: () => invoke<AppEnvironment>("app_environment"),
  setLanguage: (language: string) => invoke<void>("set_language", { language }),
  readClipboard: () => readText(),
  onUpdate: (handler: (item: DownloadItem) => void) =>
    listen<DownloadItem>("download:update", (e) => handler(e.payload)),
  onRemoved: (handler: (id: string) => void) => listen<string>("download:removed", (e) => handler(e.payload)),
};
