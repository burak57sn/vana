import { useEffect, useMemo, useState } from "react";
import { formatBytes, formatDuration } from "../format";
import { useI18n } from "../i18n";
import type { AudioFormat, DownloadRequest, MediaInfo, Selection, VideoContainer, VideoOption } from "../types";
import { Badge, Field, Segmented, Thumbnail } from "./ui";

const VIDEO_FORMATS = ["mp4", "mkv"] as const;
const AUDIO_FORMATS = ["mp3", "m4a", "wav"] as const;
const SELECT = "h-11 w-full rounded-md border border-ink-700 bg-ink-950 pl-3 pr-9 text-[15px] text-ink-100";
const DEFAULT_HEIGHT = 1080;

interface MediaPanelProps {
  media: MediaInfo;
  onDownload: (requests: DownloadRequest[]) => void;
  onOpenPlaylist: (url: string) => void;
}

function videoSize(option: VideoOption | undefined, container: VideoContainer) {
  if (!option) return null;
  return container === "mp4" ? option.sizeMp4 : option.sizeMkv;
}

function defaultOption(options: VideoOption[]) {
  return options.find((o) => Math.min(o.height, o.width ?? o.height) <= DEFAULT_HEIGHT) ?? options[0];
}

export function MediaPanel({ media, onDownload, onOpenPlaylist }: MediaPanelProps) {
  const { t, language } = useI18n();
  const hasVideo = media.videoOptions.length > 0;
  const [tab, setTab] = useState<"video" | "audio">(hasVideo ? "video" : "audio");
  const [container, setContainer] = useState<VideoContainer>("mp4");
  const [videoLabel, setVideoLabel] = useState(defaultOption(media.videoOptions)?.label ?? "");
  const [subtitles, setSubtitles] = useState(false);
  const [audioFormat, setAudioFormat] = useState<AudioFormat>("mp3");
  const [kbps, setKbps] = useState(320);
  const [picked, setPicked] = useState<Set<string>>(() => new Set(media.entries.map((e) => e.id)));
  const [queuedNote, setQueuedNote] = useState<string | null>(null);

  useEffect(() => {
    if (!queuedNote) return;
    const timer = window.setTimeout(() => setQueuedNote(null), 2500);
    return () => window.clearTimeout(timer);
  }, [queuedNote]);

  const option = media.videoOptions.find((o) => o.label === videoLabel) ?? media.videoOptions[0];
  const audioOption = media.audioOptions.find((o) => o.kbps === kbps) ?? media.audioOptions[0];
  const isWav = audioFormat === "wav";

  const estimate = useMemo(() => {
    if (media.isPlaylist) return null;
    if (tab === "video") return videoSize(option, container);
    return isWav ? media.wavSize : (audioOption?.size ?? null);
  }, [media, tab, option, container, isWav, audioOption]);

  const selection = (): Selection =>
    tab === "video"
      ? { mode: "video", container, height: option?.height ?? null, subtitles }
      : { mode: "audio", format: audioFormat, kbps: isWav ? null : kbps };

  const qualityLabel = tab === "video" ? (option?.shortLabel ?? "Best") : isWav ? "Lossless" : `${kbps}kbps`;

  const download = () => {
    const job = {
      selection: selection(),
      qualityLabel,
      estVideoBytes: tab === "video" ? ((container === "mp4" ? option?.videoBytesMp4 : option?.videoBytesMkv) ?? null) : null,
      estAudioBytes: tab === "video" ? media.audioBytes : null,
    };
    const targets = media.isPlaylist
      ? media.entries.filter((e) => picked.has(e.id)).map((e) => ({ url: e.url, title: e.title, thumbnail: e.thumbnail }))
      : [{ url: media.url, title: media.title, thumbnail: media.thumbnail }];
    onDownload(targets.map((t) => ({ job: { ...job, url: t.url }, title: t.title, thumbnail: t.thumbnail, platform: media.platform })));
    setQueuedNote(t.queued(targets.length));
  };

  const toggle = (id: string) =>
    setPicked((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const allPicked = picked.size === media.entries.length;
  const disabled = (media.isPlaylist && picked.size === 0) || (tab === "video" && !media.isPlaylist && !option);
  const highCodecNote = tab === "video" && container === "mp4" && !media.isPlaylist && option && Math.min(option.height, option.width ?? option.height) > 1080;
  const sourceBelowTarget = tab === "audio" && !isWav && media.audioSourceKbps !== null && media.audioSourceKbps < kbps;

  return (
    <section aria-label={t.selectedContent} className="overflow-hidden rounded-lg border border-ink-700 bg-ink-900">
      <div className="flex gap-4 p-4">
        <Thumbnail src={media.thumbnail} className="aspect-video w-32 shrink-0 self-start sm:w-52" />
        <div className="min-w-0 flex-1">
          <div className="mb-2 flex flex-wrap items-center gap-1.5">
            <Badge tone="red">{media.platform}</Badge>
            {media.isPlaylist && <Badge>{t.playlist} · {media.entries.length}</Badge>}
          </div>
          <h2 className="line-clamp-2 text-[17px] font-semibold leading-snug">{media.title}</h2>
          <p className="mt-1 truncate text-sm text-ink-400">
            {[media.uploader, media.duration ? formatDuration(media.duration) : null].filter(Boolean).join(" · ")}
          </p>
          {media.parentPlaylistUrl && (
            <button
              type="button"
              onClick={() => onOpenPlaylist(media.parentPlaylistUrl!)}
              className="mt-1.5 text-sm font-medium text-valve-400 underline-offset-2 hover:underline"
            >
              {t.inPlaylist}
            </button>
          )}
        </div>
      </div>

      <div role="tablist" aria-label={t.downloadType} className="grid grid-cols-2 border-y border-ink-700">
        {(
          [
            ["video", t.video],
            ["audio", t.audio],
          ] as const
        ).map(([key, label]) => (
          <button
            key={key}
            type="button"
            role="tab"
            aria-selected={tab === key}
            disabled={key === "video" && !hasVideo}
            onClick={() => setTab(key)}
            className={`relative h-12 font-display text-lg font-bold uppercase tracking-widest transition-colors disabled:opacity-30 ${
              tab === key ? "bg-ink-850 text-ink-100" : "text-ink-400 hover:text-ink-100"
            }`}
          >
            {label}
            {tab === key && <span className="absolute inset-x-0 bottom-0 h-0.5 bg-valve-500" />}
          </button>
        ))}
      </div>

      <div className="p-4">
        {tab === "video" ? (
          <div className="grid gap-4 sm:grid-cols-[auto_1fr_auto]">
            <Field label={t.format}>
              <Segmented label={t.videoFormat} options={VIDEO_FORMATS} value={container} onChange={setContainer} />
            </Field>
            <Field label={media.isPlaylist ? t.maxQuality : t.quality}>
              <select aria-label={t.videoQuality} value={option?.label ?? ""} onChange={(e) => setVideoLabel(e.target.value)} className={SELECT}>
                {media.videoOptions.map((o) => (
                  <option key={o.label} value={o.label}>
                    {o.label}
                    {videoSize(o, container) ? `  (${formatBytes(videoSize(o, container)!, language)})` : ""}
                  </option>
                ))}
              </select>
            </Field>
            <Field label={t.subtitles}>
              <button
                type="button"
                aria-pressed={subtitles}
                disabled={!media.hasSubtitles}
                onClick={() => setSubtitles((s) => !s)}
                className={`h-11 w-full whitespace-nowrap rounded-md border px-4 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${
                  subtitles ? "border-ink-100 bg-ink-100 text-ink-950" : "border-ink-700 bg-ink-950 text-ink-100 hover:border-ink-600"
                }`}
              >
                {media.hasSubtitles ? (subtitles ? t.subtitlesOn : t.subtitlesAdd) : t.subtitlesNone}
              </button>
            </Field>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-[auto_1fr]">
            <Field label={t.format}>
              <Segmented label={t.audioFormat} options={AUDIO_FORMATS} value={audioFormat} onChange={setAudioFormat} />
            </Field>
            <Field label={t.audioQuality}>
              {isWav ? (
                <div className="flex h-11 items-center rounded-md border border-ink-700 bg-ink-950 px-3 text-[15px] text-ink-300">{t.wavNote}</div>
              ) : (
                <select aria-label={t.audioQuality} value={kbps} onChange={(e) => setKbps(Number(e.target.value))} className={SELECT}>
                  {media.audioOptions.map((o) => (
                    <option key={o.kbps} value={o.kbps}>
                      {t.audioLabel(o.kbps)}
                    </option>
                  ))}
                </select>
              )}
            </Field>
          </div>
        )}

        {highCodecNote && (
          <p className="mt-3 text-sm text-ink-400">{t.highCodecNote}</p>
        )}
        {sourceBelowTarget && (
          <p className="mt-3 text-sm text-ink-400">{t.sourceBelowTarget(media.audioSourceKbps!)}</p>
        )}

        {media.isPlaylist && (
          <div className="mt-4">
            <div className="mb-1.5 flex items-center justify-between">
              <span className="font-display text-[13px] font-semibold uppercase tracking-[0.12em] text-ink-400">
                {t.selectedCount(picked.size, media.entries.length)}
              </span>
              <button
                type="button"
                onClick={() => setPicked(allPicked ? new Set() : new Set(media.entries.map((e) => e.id)))}
                className="text-sm font-medium text-ink-300 hover:text-ink-100"
              >
                {allPicked ? t.clearSelection : t.selectAll}
              </button>
            </div>
            <ul className="max-h-64 divide-y divide-ink-800 overflow-y-auto rounded-md border border-ink-700 bg-ink-950">
              {media.entries.map((entry, index) => (
                <li key={entry.id}>
                  <label className="flex cursor-pointer items-center gap-3 px-3 py-2 hover:bg-ink-900">
                    <input type="checkbox" checked={picked.has(entry.id)} onChange={() => toggle(entry.id)} className="size-4 shrink-0 accent-valve-500" />
                    <span className="w-6 shrink-0 text-right font-mono text-xs text-ink-400">{index + 1}</span>
                    <span className="min-w-0 flex-1 truncate text-sm">{entry.title}</span>
                    {entry.duration !== null && <span className="shrink-0 font-mono text-xs text-ink-400">{formatDuration(entry.duration)}</span>}
                  </label>
                </li>
              ))}
            </ul>
          </div>
        )}

        <div className="mt-4 flex flex-col-reverse items-stretch gap-3 sm:flex-row sm:items-center sm:justify-end">
          <p aria-live="polite" className="text-center font-mono text-sm text-ink-300 sm:text-right">
            {queuedNote ?? (estimate ? t.approxSize(formatBytes(estimate, language)) : media.isPlaylist ? t.sizeDuringDownload : t.sizeUnknown)}
          </p>
          <button
            type="button"
            disabled={disabled}
            onClick={download}
            className="h-12 rounded-md bg-valve-500 px-10 font-display text-lg font-bold uppercase tracking-wider text-white transition-colors hover:bg-valve-400 disabled:cursor-not-allowed disabled:bg-ink-800 disabled:text-ink-400"
          >
            {media.isPlaylist ? t.downloadMany(picked.size) : t.download}
          </button>
        </div>
      </div>
    </section>
  );
}
