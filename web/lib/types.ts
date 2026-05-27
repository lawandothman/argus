export interface Stats {
  series: number;
  samples: number;
  spans: number;
  traces: number;
  logs: number;
  metric_bytes: number;
  compression_ratio: number;
  bytes_per_sample: number;
  latest_ns: number;
}

export interface QuerySample {
  labels: Record<string, string>;
  value: number;
}

export interface QueryResult {
  query: string;
  plan?: string | null;
  result?: { type: "vector" | "scalar"; samples?: QuerySample[]; value?: number };
  error?: string;
}

export interface TraceSummary {
  trace_id: string;
  operation: string;
  service: string;
  duration_ms: number;
  start_ns: number;
  failed: boolean;
}

/** A live event pushed over the WebSocket stream. */
export interface RequestEvent {
  kind: "request";
  trace_id: string;
  route: string;
  status: number;
  duration_ms: number;
  failed: boolean;
  timestamp_ns: number;
}

/** A trace the correlation engine blamed for an anomaly, with its bottleneck span. */
export interface BottleneckTrace {
  trace_id: string;
  operation: string;
  duration_ms: number;
  failed: boolean;
  bottleneck_service: string;
  bottleneck_op: string;
  bottleneck_ms: number;
}

/** `GET /api/anomalies` — detector output plus the correlated cause. */
export interface AnomalyReport {
  changepoint: {
    timestamp_ms: number;
    observed: number;
    expected: number;
    score: number;
  } | null;
  cause: {
    slowest: BottleneckTrace[];
  } | null;
}

/** Per-service aggregates over a recent window, from `GET /api/services`. */
export interface ServiceStat {
  service: string;
  calls: number;
  rps: number;
  error_rate: number;
  p50: number;
  p95: number;
  p99: number;
}

export interface ServicesResponse {
  window_ms: number;
  services: ServiceStat[];
}

/** A typed span/log attribute value, mirroring the OTel attribute model. */
export type AttributeValue = string | number | boolean | AttributeValue[];

/** One span within a trace, from `GET /api/trace/{id}`. */
export interface SpanDetail {
  span_id: string;
  parent_span_id: string | null;
  name: string;
  service: string;
  kind: string;
  status: string; // "Ok" | "Error"
  start_ns: number;
  duration_ms: number;
  attributes: Record<string, AttributeValue>;
}

/** A log line correlated to a trace. */
export interface LogLine {
  timestamp_ns: number;
  severity: string;
  body: string;
  service: string;
  trace_id: string | null;
  span_id: string | null;
}

export interface TraceDetail {
  trace_id: string;
  spans: SpanDetail[];
  logs: LogLine[];
}
