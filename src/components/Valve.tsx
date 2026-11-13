interface ValveProps {
  size?: number;
  spinning?: boolean;
  className?: string;
}

const GRIPS = Array.from({ length: 9 }, (_, i) => i * 40);
const SPOKES = [0, 120, 240];

/** Valve handwheel; same geometry as the app icon and the menu bar icon. */
export function Valve({ size = 44, spinning = false, className = "" }: ValveProps) {
  return (
    <svg
      viewBox="0 0 100 100"
      width={size}
      height={size}
      aria-hidden="true"
      className={`${spinning ? "valve-spinning" : ""} ${className}`}
    >
      <g stroke="var(--color-valve-500)" fill="none">
        {GRIPS.map((a) => (
          <line key={a} x1="50" y1="11" x2="50" y2="3" strokeWidth="8" transform={`rotate(${a} 50 50)`} />
        ))}
        <circle cx="50" cy="50" r="36" strokeWidth="9" />
        {SPOKES.map((a) => (
          <line key={a} x1="50" y1="50" x2="50" y2="14" strokeWidth="8" transform={`rotate(${a} 50 50)`} />
        ))}
      </g>
      <circle cx="50" cy="50" r="13" fill="var(--color-ink-100)" />
      <circle cx="50" cy="50" r="5.5" fill="var(--color-valve-500)" />
    </svg>
  );
}
