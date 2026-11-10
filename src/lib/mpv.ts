import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type MpvEvent =
  | { kind: 'prop'; name: string; value: string | null }
  | { kind: 'start-file' | 'file-loaded' | 'video-reconfig' | 'seek' | 'playback-restart' | 'shutdown' }
  | { kind: 'end-file'; reason: 'eof' | 'stop' | 'quit' | 'error' | 'redirect' | 'other'; error: string | null };

export const command = (...args: (string | number)[]) => invoke<void>('mpv_command', { args: args.map(String) });
export const set = (name: string, value: string | number | boolean) =>
  invoke<void>('mpv_set', { name, value: typeof value === 'boolean' ? (value ? 'yes' : 'no') : String(value) });
export const get = (name: string) => invoke<string | null>('mpv_get', { name });
export const openFiles = (files: string[], append = false) => invoke<void>('open_files', { files, append });
export const pendingFiles = () => invoke<string[]>('pending_files');
export const edgeColors = () => invoke<[number, number, number][]>('edge_colors');
export const recentFiles = () => invoke<{ path: string; start: number; saved: number }[]>('recent_files');
export const windowButtons = (visible: boolean) => invoke<void>('window_buttons', { visible });
export const detectIntro = (path: string) => invoke<{ start: number; end: number } | null>('detect_intro', { path });
export interface TranscriptInfo { path: string; lang: string; srt: string; translated: string | null }
export interface Hit { path: string; time: number; text: string }
export const transcriptFor = (path: string) => invoke<TranscriptInfo | null>('transcript_for', { path });
export const transcriptSearch = (query: string) => invoke<Hit[]>('transcript_search', { query });
export const subtitlesCancel = () => invoke<void>('subtitles_cancel');
export const savePosition = () => invoke<void>('save_position');

export function onEvent(handler: (e: MpvEvent) => void): Promise<UnlistenFn> {
  return listen<MpvEvent>('mpv', (e) => handler(e.payload));
}
