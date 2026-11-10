export type Rgb = [number, number, number];

/** Lifts a sampled edge colour into a glow: same hue, at least some light and colour. */
export function glow([r, g, b]: Rgb): Rgb {
  const max = Math.max(r, g, b) / 255;
  const min = Math.min(r, g, b) / 255;
  let h = 0;
  const l = (max + min) / 2;
  const d = max - min;
  let s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  if (d !== 0) {
    if (max === r / 255) h = ((g - b) / 255 / d) % 6;
    else if (max === g / 255) h = (b - r) / 255 / d + 2;
    else h = (r - g) / 255 / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  const L = Math.min(0.6, Math.max(0.2, l * 1.25));
  s = Math.min(1, s * 1.3);
  const c = (1 - Math.abs(2 * L - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = L - c / 2;
  const [r1, g1, b1] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return [Math.round((r1 + m) * 255), Math.round((g1 + m) * 255), Math.round((b1 + m) * 255)];
}

export interface Rect { x: number; y: number; w: number; h: number }

/** Where mpv puts the video: fitted into the window minus `margin` (a fraction) on every side. */
export function videoRect(width: number, height: number, aspect: number, margin: number): Rect {
  const aw = width * (1 - 2 * margin);
  const ah = height * (1 - 2 * margin);
  let w = aw;
  let h = aw / aspect;
  if (h > ah) {
    h = ah;
    w = ah * aspect;
  }
  return { x: (width - w) / 2, y: (height - h) / 2, w, h };
}
