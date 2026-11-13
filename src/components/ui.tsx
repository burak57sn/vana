import type { ButtonHTMLAttributes, ReactNode } from "react";

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0">
      <div className="mb-1.5 font-display text-[13px] font-semibold uppercase tracking-[0.12em] text-ink-400">{label}</div>
      {children}
    </div>
  );
}

interface SegmentedProps<T extends string> {
  label: string;
  options: readonly T[];
  value: T;
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({ label, options, value, onChange }: SegmentedProps<T>) {
  return (
    <div role="radiogroup" aria-label={label} className="flex h-11 rounded-md border border-ink-700 bg-ink-950 p-1">
      {options.map((option) => (
        <button
          key={option}
          type="button"
          role="radio"
          aria-checked={option === value}
          onClick={() => onChange(option)}
          className={`min-w-14 flex-1 rounded px-3 font-mono text-[13px] uppercase transition-colors ${
            option === value ? "bg-ink-100 text-ink-950" : "text-ink-300 hover:text-ink-100"
          }`}
        >
          {option}
        </button>
      ))}
    </div>
  );
}

export function IconButton({ children, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      {...props}
      className={`h-9 rounded border border-ink-700 bg-ink-850 px-3 text-sm font-medium text-ink-100 transition-colors hover:border-ink-600 hover:bg-ink-800 ${className}`}
    >
      {children}
    </button>
  );
}

export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "red" | "green" | "amber" }) {
  const tones = {
    neutral: "border-ink-700 text-ink-300",
    red: "border-valve-500 bg-valve-500 text-white",
    green: "border-flow-400/40 text-flow-400",
    amber: "border-amber-valve/40 text-amber-valve",
  };
  return (
    <span className={`inline-flex h-6 shrink-0 items-center rounded-sm border px-2 font-mono text-[11px] ${tones[tone]}`}>
      {children}
    </span>
  );
}

export function Thumbnail({ src, className = "" }: { src: string | null; className?: string }) {
  return (
    <div className={`relative overflow-hidden rounded bg-ink-800 ${className}`}>
      {src && (
        <img
          src={src}
          alt=""
          referrerPolicy="no-referrer"
          className="absolute inset-0 h-full w-full object-cover"
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />
      )}
    </div>
  );
}
