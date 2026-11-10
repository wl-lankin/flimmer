import { beforeEach, describe, expect, it, vi } from 'vitest';

const sent: unknown[] = [];
let relay: (e: { payload: unknown }) => void = () => {};
let mpvEvents: (e: unknown) => void = () => {};

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args?: { msg?: unknown }) => {
    if (cmd === 'together_send') sent.push(args?.msg);
  }),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_: string, h: (e: { payload: unknown }) => void) => {
    relay = h;
    return () => {};
  }),
}));
vi.mock('./mpv', () => ({
  onEvent: vi.fn(async (h: (e: unknown) => void) => {
    mpvEvents = h;
    return () => {};
  }),
  set: vi.fn(async () => {}),
  command: vi.fn(async () => {}),
}));

const { together } = await import('./together.svelte');
const { player } = await import('./player.svelte');
const mpv = await import('./mpv');

const state = (over: Record<string, unknown>) => ({ t: 'state', paused: false, time: 100, speed: 1, file: 'a.mkv', reason: 'tick', from: '2', name: 'Ines', at: 0, ...over });
const flush = () => new Promise((r) => setTimeout(r, 0));

describe('together', () => {
  beforeEach(async () => {
    sent.length = 0;
    player.idle = false;
    player.paused = false;
    player.time = 100;
    player.speed = 1;
    player.duration = 600;
    vi.mocked(mpv.set).mockClear();
    vi.mocked(mpv.command).mockClear();
    await together.open('create');
    relay({ payload: { t: 'created', room: 'K7QX2M', you: '1' } });
    await flush();
  });

  it('pauses when a peer pauses and does not echo it back', async () => {
    relay({ payload: state({ paused: true, time: 100.2, reason: 'pause' }) });
    await flush();
    expect(mpv.set).toHaveBeenCalledWith('pause', true);
    mpvEvents({ kind: 'prop', name: 'pause', value: 'yes' });
    expect(sent.filter((m) => (m as { t: string }).t === 'state')).toHaveLength(0);
  });

  it('corrects drift beyond the tolerance and ignores small drift on ticks', async () => {
    relay({ payload: state({ time: 100.5 }) });
    await flush();
    expect(mpv.command).not.toHaveBeenCalled();
    relay({ payload: state({ time: 140, reason: 'seek' }) });
    await flush();
    expect(mpv.command).toHaveBeenCalledWith('seek', expect.any(Number), 'absolute+exact');
  });

  it('sends local changes with the room open', async () => {
    await new Promise((r) => setTimeout(r, 950));
    mpvEvents({ kind: 'prop', name: 'speed', value: '1.5' });
    const last = sent.at(-1) as { t: string; reason: string };
    expect(last).toMatchObject({ t: 'state', reason: 'speed' });
  });
});
