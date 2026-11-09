// The Velocity K: a K whose blades break away from the stem — armor in
// motion. One source of truth for every size; the blades alone are the
// small mark (favicon, 16 px file lists). Colors ride the theme tokens.

export function Logo({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <rect x="4" y="4" width="5.2" height="24" rx="2" fill="var(--accent)" />
      <path d="M14 14.6 23.4 4h5.4L18.6 15.2Z" fill="var(--accent)" />
      <path d="M14 17.4 23.4 28h5.4L18.6 16.8Z" fill="var(--accent)" />
    </svg>
  );
}

/** The blades alone — the mark for tiny sizes. */
export function Blades({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <path d="M6 13.4 17 1h6.4L10.8 14.8Z" fill="var(--accent)" />
      <path d="M6 18.6 17 31h6.4L10.8 17.2Z" fill="var(--accent)" />
    </svg>
  );
}
