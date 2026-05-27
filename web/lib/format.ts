export const ms = (n: number | null) => (n == null ? "—" : `${n.toFixed(n < 10 ? 1 : 0)}`);

export const compact = (n: number | null) =>
  n == null ? "—" : Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(n);

export const pct = (n: number | null) => (n == null ? "—" : `${(n * 100).toFixed(1)}`);

export const kb = (bytes: number) => `${(bytes / 1024).toFixed(0)}`;

export const shortId = (hex: string) => hex.slice(0, 8);
