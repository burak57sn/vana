import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Downloads } from "./components/Downloads";
import { MediaPanel } from "./components/MediaPanel";
import { UrlBar } from "./components/UrlBar";
import { Valve } from "./components/Valve";
import { type Language, useI18n } from "./i18n";
import type { AppEnvironment, DownloadItem, DownloadRequest, MediaInfo } from "./types";

const errorText = (error: unknown, fallback: string) => (typeof error === "string" ? error : error instanceof Error ? error.message : fallback);

const LANGUAGES: { value: Language; label: string }[] = [
  { value: "en", label: "EN" },
  { value: "tr", label: "TR" },
];

export default function App() {
  const { t, language, setLanguage } = useI18n();
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [media, setMedia] = useState<MediaInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState<DownloadItem[]>([]);
  const [env, setEnv] = useState<AppEnvironment | null>(null);

  const upsert = useCallback((item: DownloadItem) => {
    setItems((current) => {
      const exists = current.some((i) => i.id === item.id);
      return exists ? current.map((i) => (i.id === item.id ? item : i)) : [...current, item];
    });
  }, []);

  useEffect(() => {
    void api.listDownloads().then(setItems);
    void api.environment().then(setEnv).catch(() => setEnv(null));
    const update = api.onUpdate(upsert);
    const removed = api.onRemoved((id) => setItems((current) => current.filter((i) => i.id !== id)));
    return () => {
      void update.then((off) => off());
      void removed.then((off) => off());
    };
  }, [upsert]);

  const analyze = useCallback(
    async (target: string) => {
      const value = target.trim();
      if (!value) return;
      setLoading(true);
      setError(null);
      setMedia(null);
      try {
        setMedia(await api.fetchMetadata(value));
      } catch (e) {
        setError(errorText(e, t.unexpectedError));
      } finally {
        setLoading(false);
      }
    },
    [t],
  );

  const paste = async () => {
    try {
      const text = (await api.readClipboard()).trim();
      if (text) setUrl(text);
    } catch {
      setError(t.clipboardFailed);
    }
  };

  const download = (requests: DownloadRequest[]) => {
    for (const request of requests) {
      api.startDownload(request).then(upsert).catch((e) => setError(errorText(e, t.unexpectedError)));
    }
  };

  const guarded = (action: (id: string) => Promise<void>) => (id: string) => {
    action(id).catch((e) => setError(errorText(e, t.unexpectedError)));
  };

  const anyActive = items.some((i) => i.status === "active");
  const missing = env?.tools.filter((t) => !t.available) ?? [];

  return (
    <div className="flex min-h-screen flex-col">
      <div className="hazard h-1.5 shrink-0" aria-hidden="true" />
      <main className="mx-auto flex w-full max-w-4xl flex-1 flex-col gap-6 px-4 pb-10 pt-6 sm:px-6 sm:pt-8">
        <header className="flex items-center gap-3 sm:gap-4">
          <Valve size={56} spinning={loading || anyActive} className="shrink-0" />
          <div className="min-w-0 flex-1">
            <h1 className="font-display text-4xl font-bold uppercase leading-none tracking-wide">Vana</h1>
            <p className="mt-1 text-[15px] leading-snug text-ink-300">{t.tagline}</p>
          </div>
          <div role="radiogroup" aria-label={t.language} className="flex shrink-0 self-start rounded border border-ink-700 p-0.5">
            {LANGUAGES.map((l) => (
              <button
                key={l.value}
                type="button"
                role="radio"
                aria-checked={language === l.value}
                onClick={() => setLanguage(l.value)}
                className={`h-7 rounded-sm px-2 font-mono text-xs transition-colors ${
                  language === l.value ? "bg-ink-100 text-ink-950" : "text-ink-400 hover:text-ink-100"
                }`}
              >
                {l.label}
              </button>
            ))}
          </div>
        </header>

        <UrlBar url={url} loading={loading} onChange={setUrl} onPaste={() => void paste()} onSubmit={() => void analyze(url)} />

        {missing.length > 0 && (
          <p role="alert" className="rounded-md border border-amber-valve/40 bg-ink-900 px-4 py-3 text-sm text-amber-valve">
            {env?.isMobile ? (
              t.mobileNoEngine
            ) : (
              <>
                {t.missingTools(missing.map((tool) => tool.name).join(", "))} <code className="font-mono">npm run sidecars</code>
                {t.missingToolsAfter}
              </>
            )}
          </p>
        )}

        {error && (
          <p role="alert" className="rounded-md border border-valve-500/50 bg-valve-900/40 px-4 py-3 text-sm text-ink-100">
            {error}
          </p>
        )}

        {loading && (
          <div className="flex items-center gap-3 rounded-lg border border-ink-700 bg-ink-900 p-4 text-sm text-ink-300" role="status">
            <Valve size={24} spinning />
            {t.analyzingStatus}
          </div>
        )}

        {media && (
          <MediaPanel
            key={media.url + media.isPlaylist}
            media={media}
            onDownload={download}
            onOpenPlaylist={(playlistUrl) => {
              setUrl(playlistUrl);
              void analyze(playlistUrl);
            }}
          />
        )}

        <Downloads
          items={items}
          os={env?.os}
          isMobile={env?.isMobile ?? false}
          onCancel={guarded(api.cancel)}
          onRetry={guarded(api.retry)}
          onRemove={guarded(api.remove)}
          onOpen={guarded(api.open)}
          onReveal={guarded(api.reveal)}
        />

        <footer className="mt-auto space-y-1 text-xs text-ink-600">
          <p>{t.legalNote}</p>
          {env?.downloadDir && <p className="truncate font-mono">{t.savedTo(env.downloadDir)}</p>}
        </footer>
      </main>
    </div>
  );
}
