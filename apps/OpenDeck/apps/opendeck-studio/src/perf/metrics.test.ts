import { beforeEach, describe, expect, it } from 'vitest';
import { clearMetrics, getSamples, percentile, recordLatency, summarize } from './metrics';

describe('qualification metrics', () => {
  beforeEach(clearMetrics);

  it('computes deterministic nearest-rank p95 and p99', () => {
    for (let i = 1; i <= 100; i += 1) recordLatency('nav', i);
    expect(percentile('nav', 0.95)).toBe(95);
    expect(percentile('nav', 0.99)).toBe(99);
    expect(summarize('nav')).toMatchObject({ count: 100, min: 1, max: 100, p95: 95, p99: 99 });
  });

  it('bounds retained samples so qualification cannot grow memory indefinitely', () => {
    for (let i = 0; i < 900; i += 1) recordLatency('bounded', i);
    expect(getSamples('bounded')).toHaveLength(512);
    expect(getSamples('bounded')[0]).toBe(388);
  });
});
