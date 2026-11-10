import type { Track } from './player.svelte';
import { language, t } from './i18n';

const CHANNELS: Record<number, string> = { 1: 'Mono', 2: 'Stereo', 6: '5.1', 8: '7.1' };

/** Main line of a track: language, else title, else "Spur 3". */
export function trackName(track: Track): string {
  const lang = language(track.lang);
  if (lang && track.title && !track.title.toLowerCase().includes(lang.toLowerCase())) return `${lang} · ${track.title}`;
  return lang || track.title || `${t('track')} ${track.id}`;
}

/** Small technical note: codec, channels, forced/external. */
export function trackNote(track: Track): string {
  const parts: string[] = [];
  if (track.codec) parts.push(track.codec.toUpperCase().replace('SUBRIP', 'SRT').replace('HDMV_PGS_SUBTITLE', 'PGS'));
  const ch = track['demux-channel-count'];
  if (ch) parts.push(CHANNELS[ch] ?? `${ch} ch`);
  if (track.forced) parts.push(t('forced'));
  if (track.external) parts.push(t('external'));
  return parts.join(' · ');
}
