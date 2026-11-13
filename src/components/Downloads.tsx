import { formatDuration, formatPercent, formatSpeed } from "../format";
import { type Messages, useI18n } from "../i18n";
import type { DownloadItem } from "../types";
import { Badge, IconButton, Thumbnail } from "./ui";

interface DownloadsProps {
  items: DownloadItem[];
  os?: string;
  isMobile: boolean;
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  onOpen: (id: string) => void;
  onReveal: (id: string) => void;
}

function statusLine(item: DownloadItem, t: Messages) {
  switch (item.status) {
    case "queued":
      return t.queuedStatus;
    case "completed":
      return t.completed;
    case "canceled":
      return t.canceled;
    case "failed":
      return item.error ?? t.failed;
    case "active":
      return t.stage[item.stage];
  }
}

export function Downloads({ items, os, isMobile, onCancel, onRetry, onRemove, onOpen, onReveal }: DownloadsProps) {
  const { t, language } = useI18n();
  const active = items.filter((i) => i.status === "active" || i.status === "queued").length;

  return (
    <section aria-label={t.downloads}>
      <div className="mb-3 flex items-baseline justify-between">
        <h2 className="font-display text-xl font-bold uppercase tracking-widest">{t.downloads}</h2>
        {active > 0 && <span className="font-mono text-xs text-ink-400">{t.activeCount(active)}</span>}
      </div>

      {items.length === 0 ? (
        <p className="rounded-lg border border-dashed border-ink-700 px-4 py-8 text-center text-sm text-ink-400">
          {t.noDownloads}
        </p>
      ) : (
        <ul className="space-y-2">
          {[...items].reverse().map((item) => (
            <li key={item.id} className="flex gap-3 rounded-lg border border-ink-700 bg-ink-900 p-3">
              <Thumbnail src={item.thumbnail} className="hidden aspect-video w-24 shrink-0 self-start sm:block" />
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                  <Badge tone="red">{item.tag}</Badge>
                  <Badge>{item.quality}</Badge>
                  <h3 className="min-w-0 flex-1 basis-40 truncate text-[15px] font-medium" title={item.title}>
                    {item.title}
                  </h3>
                </div>

                {(item.status === "active" || item.status === "queued") && (
                  <div
                    role="progressbar"
                    aria-valuenow={Math.round(item.percent)}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-label={item.title}
                    className="mt-2.5 h-2 overflow-hidden rounded-sm bg-ink-800"
                  >
                    <div className="progress-flow h-full" style={{ width: `${item.percent}%` }} />
                  </div>
                )}

                <div className="mt-2 flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
                  <div className="flex min-w-0 flex-wrap items-baseline gap-x-3 font-mono text-xs">
                    <span className={item.status === "failed" ? "font-sans text-sm text-valve-400" : item.status === "completed" ? "text-flow-400" : "text-ink-300"}>
                      {statusLine(item, t)}
                    </span>
                    {item.status === "active" && <span className="text-ink-100">{formatPercent(item.percent, language)}</span>}
                    {item.status === "active" && item.speedBps !== null && <span className="text-ink-300">{formatSpeed(item.speedBps, language)}</span>}
                    {item.status === "active" && item.etaSecs !== null && <span className="text-ink-400">{t.remaining(formatDuration(item.etaSecs))}</span>}
                  </div>

                  <div className="flex flex-wrap gap-1.5">
                    {(item.status === "active" || item.status === "queued") && <IconButton onClick={() => onCancel(item.id)}>{t.cancel}</IconButton>}
                    {item.status === "completed" && <IconButton onClick={() => onOpen(item.id)}>{t.open}</IconButton>}
                    {item.status === "completed" && !isMobile && <IconButton onClick={() => onReveal(item.id)}>{os === "macos" ? t.revealFinder : t.revealFolder}</IconButton>}
                    {(item.status === "failed" || item.status === "canceled") && <IconButton onClick={() => onRetry(item.id)}>{t.retry}</IconButton>}
                    {item.status !== "active" && item.status !== "queued" && (
                      <IconButton onClick={() => onRemove(item.id)} aria-label={t.removeLabel(item.title)}>
                        {t.remove}
                      </IconButton>
                    )}
                  </div>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
