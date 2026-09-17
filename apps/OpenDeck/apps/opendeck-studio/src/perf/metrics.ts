const MAX_SAMPLES = 512;
const samples = new Map<string, number[]>();

export interface MetricSummary {
  count: number;
  min: number;
  max: number;
  mean: number;
  p50: number;
  p95: number;
  p99: number;
}

export function clearMetrics(): void {
  samples.clear();
}

export function recordLatency(name: string, durationMs: number): void {
  if (!Number.isFinite(durationMs) || durationMs < 0) return;
  const current = samples.get(name) ?? [];
  current.push(durationMs);
  if (current.length > MAX_SAMPLES) current.splice(0, current.length - MAX_SAMPLES);
  samples.set(name, current);
}

export function getSamples(name: string): readonly number[] {
  return samples.get(name) ?? [];
}

export function percentile(name: string, p: number): number {
  const values = [...(samples.get(name) ?? [])].sort((a, b) => a - b);
  if (values.length === 0) return 0;
  const bounded = Math.min(1, Math.max(0, p));
  const rank = Math.max(1, Math.ceil(bounded * values.length));
  return values[Math.min(values.length - 1, rank - 1)];
}

export function summarize(name: string): MetricSummary {
  const values = samples.get(name) ?? [];
  if (values.length === 0) return { count: 0, min: 0, max: 0, mean: 0, p50: 0, p95: 0, p99: 0 };
  const total = values.reduce((sum, value) => sum + value, 0);
  return {
    count: values.length,
    min: Math.min(...values),
    max: Math.max(...values),
    mean: total / values.length,
    p50: percentile(name, 0.50),
    p95: percentile(name, 0.95),
    p99: percentile(name, 0.99),
  };
}

export function metricSnapshot(): Record<string, MetricSummary> {
  return Object.fromEntries([...samples.keys()].sort().map((name) => [name, summarize(name)]));
}
