import * as mpv from './mpv';
import type { MpvEvent } from './mpv';

export interface Track {
  id: number;
  type: 'video' | 'audio' | 'sub';
  title?: string;
  lang?: string;
  codec?: string;
  default?: boolean;
  forced?: boolean;
  external?: boolean;
  selected?: boolean;
  image?: boolean;
  'demux-w'?: number;
  'demux-h'?: number;
  'demux-fps'?: number;
  'demux-channel-count'?: number;
  'demux-channels'?: string;
  'demux-samplerate'?: number;
  'codec-desc'?: string;
  'codec-profile'?: string;
}

export interface Chapter {
  title?: string;
  time: number;
}

export interface VideoParams {
  w?: number;
  h?: number;
  dw?: number;
  dh?: number;
  pixelformat?: string;
  'hw-pixelformat'?: string;
  primaries?: string;
  gamma?: string;
  colormatrix?: string;
  'sig-peak'?: number;
  rotate?: number;
}

function num(v: string | null): number {
  const n = v === null ? NaN : parseFloat(v);
  return Number.isFinite(n) ? n : 0;
}

function json<T>(v: string | null, fallback: T): T {
  if (!v) return fallback;
  try {
    return JSON.parse(v) as T;
  } catch {
    return fallback;
  }
}

/** The player as the page sees it: every observed mpv property, parsed. */
class Player {
  idle = $state(true);
  path = $state('');
  title = $state('');
  time = $state(0);
  duration = $state(0);
  paused = $state(false);
  eof = $state(false);
  volume = $state(100);
  muted = $state(false);
  speed = $state(1);
  tracks = $state<Track[]>([]);
  chapters = $state<Chapter[]>([]);
  chapter = $state(-1);
  video = $state<VideoParams>({});
  videoCodec = $state('');
  audioCodec = $state('');
  hwdec = $state('');
  cacheTime = $state(0);
  buffering = $state(false);
  subDelay = $state(0);
  audioDelay = $state(0);
  loopA = $state<number | null>(null);
  loopB = $state<number | null>(null);
  fps = $state(0);
  playlistPos = $state(-1);
  playlistCount = $state(0);
  /** Bumped on every seek, so views can react to jumps. */
  seekCount = $state(0);
  lastError = $state<string | null>(null);

  hasVideo = $derived(this.tracks.some((t) => t.type === 'video' && t.selected && !t.image) && (this.video.dw ?? 0) > 0);
  audioTracks = $derived(this.tracks.filter((t) => t.type === 'audio'));
  subTracks = $derived(this.tracks.filter((t) => t.type === 'sub'));
  videoTracks = $derived(this.tracks.filter((t) => t.type === 'video' && !t.image));
  currentAudio = $derived(this.audioTracks.find((t) => t.selected));
  currentSub = $derived(this.subTracks.find((t) => t.selected));
  remaining = $derived(Math.max(0, this.duration - this.time));
  aspect = $derived(this.video.dw && this.video.dh ? this.video.dw / this.video.dh : 16 / 9);
  hdr = $derived(this.video.gamma === 'pq' || this.video.gamma === 'hlg');
  currentChapter = $derived(this.chapter >= 0 ? this.chapters[this.chapter] : undefined);

  handle(e: MpvEvent) {
    switch (e.kind) {
      case 'prop':
        this.prop(e.name, e.value);
        break;
      case 'start-file':
        this.lastError = null;
        break;
      case 'seek':
        this.seekCount++;
        break;
      case 'end-file':
        if (e.reason === 'error') this.lastError = e.error ?? 'unknown error';
        break;
    }
  }

  private prop(name: string, v: string | null) {
    switch (name) {
      case 'time-pos': this.time = num(v); break;
      case 'duration': this.duration = num(v); break;
      case 'pause': this.paused = v === 'yes'; break;
      case 'idle-active': this.idle = v === 'yes'; break;
      case 'eof-reached': this.eof = v === 'yes'; break;
      case 'volume': this.volume = num(v); break;
      case 'mute': this.muted = v === 'yes'; break;
      case 'speed': this.speed = num(v) || 1; break;
      case 'media-title': this.title = v ?? ''; break;
      case 'path': this.path = v ?? ''; break;
      case 'track-list': this.tracks = json<Track[]>(v, []); break;
      case 'chapter-list': this.chapters = json<Chapter[]>(v, []); break;
      case 'chapter': this.chapter = v === null ? -1 : num(v); break;
      case 'video-params': this.video = json<VideoParams>(v, {}); break;
      case 'video-format': this.videoCodec = v ?? ''; break;
      case 'audio-codec-name': this.audioCodec = v ?? ''; break;
      case 'hwdec-current': this.hwdec = v ?? ''; break;
      case 'demuxer-cache-time': this.cacheTime = num(v); break;
      case 'paused-for-cache': this.buffering = v === 'yes'; break;
      case 'sub-delay': this.subDelay = num(v); break;
      case 'audio-delay': this.audioDelay = num(v); break;
      case 'ab-loop-a': this.loopA = v === null || v === 'no' ? null : num(v); break;
      case 'ab-loop-b': this.loopB = v === null || v === 'no' ? null : num(v); break;
      case 'estimated-vf-fps': this.fps = num(v); break;
      case 'playlist-pos': this.playlistPos = v === null ? -1 : num(v); break;
      case 'playlist-count': this.playlistCount = num(v); break;
    }
  }

  togglePause() {
    if (this.eof) return this.replay();
    return mpv.command('cycle', 'pause');
  }
  replay() {
    return mpv.command('seek', 0, 'absolute').then(() => mpv.set('pause', false));
  }
  seek(seconds: number, exact = true) {
    const t = Math.max(0, Math.min(this.duration || seconds, seconds));
    this.time = t;
    return mpv.command('seek', t, exact ? 'absolute+exact' : 'absolute+keyframes');
  }
  seekBy(delta: number) {
    return mpv.command('seek', delta, 'relative+exact');
  }
  frameStep(forward: boolean) {
    return mpv.command(forward ? 'frame-step' : 'frame-back-step');
  }
  setVolume(v: number) {
    this.volume = Math.max(0, Math.min(130, v));
    return mpv.set('volume', this.volume.toFixed(0));
  }
  toggleMute() {
    return mpv.command('cycle', 'mute');
  }
  setSpeed(s: number) {
    this.speed = Math.max(0.25, Math.min(4, Math.round(s * 100) / 100));
    return mpv.set('speed', this.speed);
  }
  selectTrack(type: 'audio' | 'sub' | 'video', id: number | 'no') {
    const prop = type === 'audio' ? 'aid' : type === 'sub' ? 'sid' : 'vid';
    return mpv.set(prop, id === 'no' ? 'no' : String(id));
  }
  cycleTrack(type: 'audio' | 'sub') {
    return mpv.command('cycle', type);
  }
  addSubtitle(path: string) {
    return mpv.command('sub-add', path, 'select');
  }
  setSubDelay(seconds: number) {
    return mpv.set('sub-delay', seconds.toFixed(2));
  }
  /** A, then B, then off, like mpv's own `l` key. */
  cycleLoop() {
    return mpv.command('ab-loop');
  }
  screenshot() {
    return mpv.command('screenshot', 'video');
  }
  nextChapter(delta: number) {
    return mpv.command('add', 'chapter', delta);
  }
}

export const player = new Player();
