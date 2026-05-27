import type {
  AnomalyReport,
  QueryResult,
  ServicesResponse,
  Stats,
  TraceDetail,
  TraceSummary,
} from "./types";

const BASE = process.env.NEXT_PUBLIC_ARGUS_URL ?? "http://localhost:8080";

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { cache: "no-store" });
  if (!res.ok) throw new Error(`${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

export const api = {
  stats: () => get<Stats>("/api/stats"),
  query: (q: string) => get<QueryResult>(`/api/query?q=${encodeURIComponent(q)}`),
  traces: (limit = 20) => get<{ traces: TraceSummary[] }>(`/api/traces?limit=${limit}`),
  trace: (id: string) => get<TraceDetail>(`/api/trace/${id}`),
  anomalies: () => get<AnomalyReport>("/api/anomalies"),
  services: () => get<ServicesResponse>("/api/services"),
};

/** WebSocket URL for the live event stream. */
export const streamUrl = () => `${BASE.replace(/^http/, "ws")}/api/stream`;

/** First value out of a query result (scalar, or the first vector sample). */
export function scalarOf(result: QueryResult | undefined): number | null {
  if (!result?.result) return null;
  if (result.result.type === "scalar") return result.result.value ?? null;
  return result.result.samples?.[0]?.value ?? null;
}
