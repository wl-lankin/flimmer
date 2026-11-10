import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import * as mpv from './mpv';
import { player } from './player.svelte';
import { ui } from './ui.svelte';
import { clock, fileName } from './format';
import { t } from './i18n';

type Reason = 'play' | 'pause' | 'seek' | 'speed' | 'tick' | 'load';
interface Peer { id: string; name: string }
interface State { paused: boolean; time: number; speed: number; file: string; reason: Reason; from: string; name: string; at: number }
type Incoming =
  | { t: 'created' | 'joined'; room: string; you: string }
  | { t: 'peers'; peers: Peer[] }
  | ({ t: 'state' } & State)
  | { t: 'pong'; at: number }
  | { t: 'error'; code: string }
  | { t: 'closed' };

/** Drift beyond this is corrected with a seek. */
const TOLERANCE = 1.2;
/** Changes we make because of a peer are not sent back. */
const ECHO_GUARD_MS = 900;

function load(key: string, fallback: string) {
  try {
    return localStorage.getItem('flimmer.' + key) ?? fallback;
  } catch {
    return fallback;
  }
}

class Together {
  room = $state<string | null>(null);
  me = $state<string | null>(null);
  peers = $state<Peer[]>([]);
  connecting = $state(false);
  name = $state(load('together.name', ''));
  #applyingUntil = 0;
  #rtt = 0;
  #pingSent = 0;
  #timer: ReturnType<typeof setInterval> | undefined;
  #started = false;

  start() {
    if (this.#started) return;
    this.#started = true;
    listen<Incoming>('together', (e) => this.receive(e.payload));
    mpv.onEvent((e) => {
      if (!this.room || Date.now() < this.#applyingUntil) return;
      if (e.kind === 'prop' && e.name === 'pause') this.send(e.value === 'yes' ? 'pause' : 'play');
      else if (e.kind === 'prop' && e.name === 'speed') this.send('speed');
      else if (e.kind === 'playback-restart') this.send('seek');
      else if (e.kind === 'file-loaded') this.send('load');
    });
  }

  rememberName(name: string) {
    this.name = name;
    try {
      localStorage.setItem('flimmer.together.name', name);
    } catch {
      /* ignore */
    }
  }

  async open(action: 'create' | 'join', room = '') {
    this.start();
    this.connecting = true;
    try {
      await invoke('together_open', { action, room: room.trim().toUpperCase(), name: this.name.trim() });
    } catch (e) {
      this.connecting = false;
      ui.say(`${t('togetherError')}: ${e}`);
    }
  }

  async leave() {
    await invoke('together_close').catch(() => {});
    this.reset();
  }

  private reset() {
    this.room = null;
    this.me = null;
    this.peers = [];
    this.connecting = false;
    clearInterval(this.#timer);
  }

  private send(reason: Reason) {
    if (!this.room) return;
    const msg = { t: 'state', paused: player.paused, time: player.time, speed: player.speed, file: fileName(player.path), reason };
    invoke('together_send', { msg }).catch(() => {});
  }

  private nameOf(id: string) {
    return this.peers.find((p) => p.id === id)?.name ?? '';
  }

  private async receive(m: Incoming) {
    switch (m.t) {
      case 'created':
      case 'joined':
        this.room = m.room;
        this.me = m.you;
        this.connecting = false;
        clearInterval(this.#timer);
        this.#timer = setInterval(() => {
          if (!player.paused) this.send('tick');
          this.#pingSent = Date.now();
          invoke('together_send', { msg: { t: 'ping' } }).catch(() => {});
        }, 5000);
        if (m.t === 'joined') this.send('tick');
        break;
      case 'peers': {
        const before = new Set(this.peers.map((p) => p.id));
        this.peers = m.peers;
        for (const p of m.peers) if (!before.has(p.id) && p.id !== this.me && before.size) ui.say(t('joined_room').replace('{name}', p.name));
        break;
      }
      case 'pong':
        this.#rtt = Date.now() - this.#pingSent;
        break;
      case 'error':
        this.connecting = false;
        ui.say(m.code === 'room-not-found' ? t('roomNotFound') : m.code === 'room-full' ? t('roomFull') : t('togetherError'));
        break;
      case 'closed':
        if (this.room) ui.say(t('togetherError'));
        this.reset();
        break;
      case 'state':
        await this.apply(m);
        break;
    }
  }

  /** Brings the local player to a peer's state, estimating where the peer is by now. */
  private async apply(s: State) {
    if (player.idle) return;
    const expected = s.paused ? s.time : s.time + (this.#rtt / 2000) * s.speed;
    const drift = Math.abs(player.time - expected);
    const guard = () => (this.#applyingUntil = Date.now() + ECHO_GUARD_MS);
    if (s.reason === 'tick' && (s.paused || drift < TOLERANCE * 2)) return;
    if (Math.abs(player.speed - s.speed) > 0.001) {
      guard();
      await player.setSpeed(s.speed);
    }
    if (drift > TOLERANCE || s.reason === 'seek') {
      guard();
      await player.seek(expected);
      if (s.reason === 'seek') ui.say(t('seeked_by').replace('{name}', s.name).replace('{time}', clock(expected)));
    }
    if (s.paused !== player.paused) {
      guard();
      await mpv.set('pause', s.paused);
      ui.say((s.paused ? t('paused_by') : t('played_by')).replace('{name}', s.name || this.nameOf(s.from)));
    }
  }
}

export const together = new Together();
