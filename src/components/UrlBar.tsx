import type { FormEvent } from "react";
import { useI18n } from "../i18n";

interface UrlBarProps {
  url: string;
  loading: boolean;
  onChange: (url: string) => void;
  onPaste: () => void;
  onSubmit: () => void;
}

export function UrlBar({ url, loading, onChange, onPaste, onSubmit }: UrlBarProps) {
  const { t } = useI18n();
  const submit = (e: FormEvent) => {
    e.preventDefault();
    onSubmit();
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-2 sm:flex-row">
      <div className="flex h-13 min-w-0 flex-1 items-center rounded-md border border-ink-700 bg-ink-900 pr-1.5 focus-within:border-valve-500">
        <input
          type="url"
          inputMode="url"
          value={url}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t.urlPlaceholder}
          aria-label={t.urlLabel}
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          className="h-full min-w-0 flex-1 bg-transparent px-4 text-[15px] outline-none placeholder:text-ink-600"
        />
        <button
          type="button"
          onClick={onPaste}
          className="h-10 shrink-0 rounded border border-ink-700 bg-ink-850 px-3.5 text-sm font-medium hover:bg-ink-800"
        >
          {t.paste}
        </button>
      </div>
      <button
        type="submit"
        disabled={loading || !url.trim()}
        className="h-13 shrink-0 rounded-md bg-valve-500 px-8 font-display text-lg font-bold uppercase tracking-wider text-white transition-colors hover:bg-valve-400 disabled:cursor-not-allowed disabled:bg-ink-800 disabled:text-ink-400"
      >
        {loading ? t.analyzing : t.fetch}
      </button>
    </form>
  );
}
