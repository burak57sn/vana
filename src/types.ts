export type VideoContainer = "mp4" | "mkv";
export type AudioFormat = "mp3" | "m4a" | "wav";

export type Selection =
  | { mode: "video"; container: VideoContainer; height: number | null; subtitles: boolean }
  | { mode: "audio"; format: AudioFormat; kbps: number | null };

export interface VideoOption {
  height: number;
  width: number | null;
  label: string;
  shortLabel: string;
  fps: number | null;
  sizeMp4: number | null;
  sizeMkv: number | null;
  videoBytesMp4: number | null;
  videoBytesMkv: number | null;
}

export interface AudioOption {
  kbps: number;
  label: string;
  size: number | null;
}

export interface PlaylistEntry {
  id: string;
  url: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
}

export interface MediaInfo {
  isPlaylist: boolean;
  url: string;
  platform: string;
  title: string;
  uploader: string | null;
  thumbnail: string | null;
  duration: number | null;
  videoOptions: VideoOption[];
  audioOptions: AudioOption[];
  wavSize: number | null;
  audioSourceKbps: number | null;
  audioBytes: number | null;
  hasSubtitles: boolean;
  entries: PlaylistEntry[];
  parentPlaylistUrl: string | null;
}

export type Status = "queued" | "active" | "completed" | "failed" | "canceled";
export type Stage = "downloading" | "merging" | "converting" | "embedding" | "finishing";

export interface DownloadItem {
  id: string;
  title: string;
  thumbnail: string | null;
  platform: string;
  tag: string;
  quality: string;
  status: Status;
  stage: Stage;
  percent: number;
  speedBps: number | null;
  etaSecs: number | null;
  error: string | null;
  filePath: string | null;
}

export interface DownloadJob {
  url: string;
  selection: Selection;
  qualityLabel: string;
  estVideoBytes: number | null;
  estAudioBytes: number | null;
}

export interface DownloadRequest {
  job: DownloadJob;
  title: string;
  thumbnail: string | null;
  platform: string;
}

export interface AppEnvironment {
  os: string;
  isMobile: boolean;
  engine: string;
  downloadDir: string | null;
  tools: { name: string; available: boolean; version: string | null }[];
}
