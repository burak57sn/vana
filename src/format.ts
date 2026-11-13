import type { Language } from "./i18n";

const LOCALES: Record<Language, string> = { en: "en-US", tr: "tr-TR" };

function number(value: number, language: Language, digits: number) {
  return new Intl.NumberFormat(LOCALES[language], { maximumFractionDigits: digits }).format(value);
}

export function formatBytes(bytes: number, language: Language): string {
  if (bytes >= 1e9) return `${number(bytes / 1e9, language, 1)} GB`;
  if (bytes >= 1e6) return `${number(bytes / 1e6, language, 1)} MB`;
  return `${number(Math.max(bytes, 1000) / 1e3, language, 0)} KB`;
}

export function formatSpeed(bps: number, language: Language): string {
  const unit = language === "tr" ? "/sn" : "/s";
  if (bps >= 1e6) return `${number(bps / 1e6, language, 1)} MB${unit}`;
  return `${number(bps / 1e3, language, 0)} KB${unit}`;
}

export function formatDuration(totalSeconds: number): string {
  const s = Math.round(totalSeconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}

export function formatPercent(percent: number, language: Language): string {
  const value = number(percent, language, 0);
  return language === "tr" ? `%${value}` : `${value}%`;
}
