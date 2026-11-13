import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { Stage } from "./types";

export type Language = "en" | "tr";

const STORAGE_KEY = "vana.language";

const en = {
  tagline: "Paste a link, open the valve, let Vana handle the rest.",
  urlPlaceholder: "YouTube, TikTok, Instagram or X link",
  urlLabel: "Media link",
  paste: "Paste",
  fetch: "Fetch",
  analyzing: "Analyzing",
  analyzingStatus: "Analyzing the link. The first run can take a few seconds.",
  clipboardFailed: "Could not read the clipboard. You can paste the link manually.",
  unexpectedError: "An unexpected error occurred.",
  mobileNoEngine: "The Android download engine is not in this version yet. You can try the interface, but fetching and downloading won't work.",
  missingTools: (tools: string) => `Missing tool: ${tools}. In a development setup, run`,
  missingToolsAfter: ".",
  savedTo: (dir: string) => `Saving to: ${dir}`,
  legalNote: "Only download content you own or have permission to download. Respect copyright and each platform's terms.",
  language: "Language",

  selectedContent: "Selected content",
  playlist: "Playlist",
  inPlaylist: "This video is in a playlist. Fetch the whole playlist",
  downloadType: "Download type",
  video: "Video",
  audio: "Audio",
  format: "Format",
  videoFormat: "Video format",
  audioFormat: "Audio format",
  quality: "Quality",
  maxQuality: "Maximum quality",
  videoQuality: "Video quality",
  audioQuality: "Audio quality",
  subtitles: "Subtitles",
  subtitlesOn: "Subtitles will be added",
  subtitlesAdd: "Add subtitles",
  subtitlesNone: "No subtitles",
  wavNote: "Lossless PCM, no bitrate to choose",
  audioLabel: (kbps: number) =>
    ({ 320: "320 kbps Studio", 256: "256 kbps High", 192: "192 kbps Normal", 128: "128 kbps Small" })[kbps] ?? `${kbps} kbps`,
  highCodecNote: "At this quality the source is usually VP9 or AV1. QuickTime may not open it; VLC will.",
  sourceBelowTarget: (kbps: number) => `The source audio is about ${kbps} kbps. A higher bitrate makes the file bigger, not better.`,
  selectedCount: (picked: number, total: number) => `${picked} / ${total} selected`,
  selectAll: "Select all",
  clearSelection: "Clear selection",
  queued: (count: number) => (count > 1 ? `${count} items added to the queue` : "Added to the queue"),
  approxSize: (size: string) => `about ${size}`,
  sizeDuringDownload: "size known while downloading",
  sizeUnknown: "size unknown",
  downloadMany: (count: number) => `Download ${count} ${count === 1 ? "item" : "items"}`,
  download: "Download",

  downloads: "Downloads",
  activeCount: (n: number) => `${n} active`,
  noDownloads: "No downloads yet. Your files will show up here.",
  stage: {
    downloading: "Downloading",
    merging: "Merging video and audio",
    converting: "Converting",
    embedding: "Adding cover and details",
    finishing: "Finishing",
  } satisfies Record<Stage, string>,
  queuedStatus: "Waiting in queue",
  completed: "Completed",
  canceled: "Canceled",
  failed: "Download failed",
  remaining: (time: string) => `${time} left`,
  cancel: "Cancel",
  open: "Open",
  revealFinder: "Show in Finder",
  revealFolder: "Show in folder",
  retry: "Retry",
  remove: "Remove",
  removeLabel: (title: string) => `Remove ${title} from the list`,
};

export type Messages = typeof en;

const tr: Messages = {
  tagline: "Bağlantıyı yapıştır, vanayı aç, gerisini Vana halletsin.",
  urlPlaceholder: "YouTube, TikTok, Instagram veya X bağlantısı",
  urlLabel: "Video bağlantısı",
  paste: "Yapıştır",
  fetch: "Getir",
  analyzing: "Analiz ediliyor",
  analyzingStatus: "Bağlantı analiz ediliyor. İlk açılışta birkaç saniye sürebilir.",
  clipboardFailed: "Pano okunamadı. Bağlantıyı elle yapıştırabilirsin.",
  unexpectedError: "Beklenmeyen bir hata oluştu.",
  mobileNoEngine: "Android indirme motoru bu sürümde henüz yok. Arayüzü deneyebilirsin ama bağlantı getirme ve indirme çalışmaz.",
  missingTools: (tools) => `Eksik araç: ${tools}. Geliştirme ortamında`,
  missingToolsAfter: " komutunu çalıştır.",
  savedTo: (dir) => `Kayıt yeri: ${dir}`,
  legalNote: "Yalnızca sana ait olan veya indirme izni bulunan içerikleri indir. Telif haklarına ve platform kurallarına uy.",
  language: "Dil",

  selectedContent: "Seçilen içerik",
  playlist: "Oynatma listesi",
  inPlaylist: "Bu video bir listede. Tüm listeyi getir",
  downloadType: "İndirme türü",
  video: "Video",
  audio: "Ses",
  format: "Format",
  videoFormat: "Video formatı",
  audioFormat: "Ses formatı",
  quality: "Kalite",
  maxQuality: "En yüksek kalite",
  videoQuality: "Video kalitesi",
  audioQuality: "Ses kalitesi",
  subtitles: "Altyazı",
  subtitlesOn: "Altyazı eklenecek",
  subtitlesAdd: "Altyazıları ekle",
  subtitlesNone: "Altyazı yok",
  wavNote: "Kayıpsız PCM, bit hızı seçilmez",
  audioLabel: (kbps) =>
    ({ 320: "320 kbps Stüdyo", 256: "256 kbps Yüksek", 192: "192 kbps Normal", 128: "128 kbps Düşük Boyut" })[kbps] ?? `${kbps} kbps`,
  highCodecNote: "Bu kalitede kaynak genellikle VP9 veya AV1'dir. QuickTime açamayabilir, VLC ile açılır.",
  sourceBelowTarget: (kbps) => `Kaynak ses yaklaşık ${kbps} kbps. Daha yüksek bit hızı dosyayı büyütür, kaliteyi artırmaz.`,
  selectedCount: (picked, total) => `${picked} / ${total} seçili`,
  selectAll: "Tümünü seç",
  clearSelection: "Seçimi kaldır",
  queued: (count) => (count > 1 ? `${count} öğe kuyruğa eklendi` : "Kuyruğa eklendi"),
  approxSize: (size) => `yaklaşık ${size}`,
  sizeDuringDownload: "boyut indirirken belli olur",
  sizeUnknown: "boyut bilinmiyor",
  downloadMany: (count) => `${count} öğeyi indir`,
  download: "İndir",

  downloads: "İndirilenler",
  activeCount: (n) => `${n} aktif`,
  noDownloads: "Henüz indirme yok. İndirdiğin dosyalar burada görünür.",
  stage: {
    downloading: "İndiriliyor",
    merging: "Video ve ses birleştiriliyor",
    converting: "Dönüştürülüyor",
    embedding: "Kapak ve bilgiler ekleniyor",
    finishing: "Tamamlanıyor",
  },
  queuedStatus: "Sırada bekliyor",
  completed: "Tamamlandı",
  canceled: "İptal edildi",
  failed: "İndirme başarısız",
  remaining: (time) => `kalan ${time}`,
  cancel: "İptal",
  open: "Aç",
  revealFinder: "Finder'da göster",
  revealFolder: "Klasörde göster",
  retry: "Tekrar dene",
  remove: "Kaldır",
  removeLabel: (title) => `${title} listeden kaldır`,
};

const MESSAGES: Record<Language, Messages> = { en, tr };

/** English unless the user picked Turkish with the EN/TR switch. */
function initialLanguage(): Language {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "en" || stored === "tr") return stored;
  } catch {
    // Storage may be unavailable; use the default.
  }
  return "en";
}

interface I18n {
  language: Language;
  setLanguage: (language: Language) => void;
  t: Messages;
}

const I18nContext = createContext<I18n | null>(null);

/** `onChange` tells the backend so errors and the tray menu use the same language. */
export function I18nProvider({ children, onChange }: { children: ReactNode; onChange?: (language: Language) => void }) {
  const [language, setLanguageState] = useState<Language>(initialLanguage);

  useEffect(() => {
    document.documentElement.lang = language;
    onChange?.(language);
  }, [language, onChange]);

  const setLanguage = useCallback((next: Language) => {
    setLanguageState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // The choice then lasts only for this session.
    }
  }, []);

  const value = useMemo(() => ({ language, setLanguage, t: MESSAGES[language] }), [language, setLanguage]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n must be used inside I18nProvider");
  return context;
}
