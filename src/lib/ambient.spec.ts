import { describe, expect, it } from 'vitest';
import { glow, videoRect } from './ambient';

describe('ambient', () => {
  it('keeps the hue and lifts dark colours', () => {
    const [r, g, b] = glow([40, 10, 10]);
    expect(r).toBeGreaterThan(g);
    expect(r).toBeGreaterThan(b);
    expect((Math.max(r, g, b) + Math.min(r, g, b)) / 2 / 255).toBeGreaterThanOrEqual(0.19);
  });
  it('grey stays grey', () => {
    const [r, g, b] = glow([128, 128, 128]);
    expect(r).toBe(g);
    expect(g).toBe(b);
  });
  it('fits wide video with bars top and bottom', () => {
    expect(videoRect(1000, 1000, 2, 0)).toEqual({ x: 0, y: 250, w: 1000, h: 500 });
  });
  it('honours the margin', () => {
    const r = videoRect(1000, 500, 2, 0.05);
    expect(r.h).toBeCloseTo(450);
    expect(r.w).toBeCloseTo(900);
    expect(r.x).toBeCloseTo(50);
  });
});
