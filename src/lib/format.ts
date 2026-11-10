const pad = (n: number) => String(Math.floor(n)).padStart(2, '0');

/** 41:12, or 1:02:03 once the media is an hour or longer. */
export function clock(sec: number, long = sec >= 3600): string {
  if (!Number.isFinite(sec) || sec < 0) sec = 0;
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  return long ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** Broadcast timecode with frames: 00:41:12:08. */
export function timecode(sec: number, fps: number): string {
  if (!Number.isFinite(sec) || sec < 0) sec = 0;
  const frames = fps > 0 ? Math.floor((sec % 1) * fps + 1e-6) : 0;
  return `${pad(sec / 3600)}:${pad((sec % 3600) / 60)}:${pad(sec % 60)}:${pad(frames)}`;
}

/** "1 Std. 10 Min." / "1 h 10 min"; under a minute "unter 1 Min.". */
export function humanDuration(sec: number, lang: 'de' | 'en'): string {
  const total = Math.max(0, Math.round(sec / 60));
  const h = Math.floor(total / 60);
  const m = total % 60;
  if (lang === 'de') {
    if (total === 0) return 'unter 1 Min.';
    return [h ? `${h} Std.` : '', m ? `${m} Min.` : ''].filter(Boolean).join(' ');
  }
  if (total === 0) return 'under 1 min';
  return [h ? `${h} h` : '', m ? `${m} min` : ''].filter(Boolean).join(' ');
}

/** Wall-clock time the media ends at, given what is left and the playback speed. */
export function endsAt(remaining: number, speed: number, now = new Date()): string {
  const end = new Date(now.getTime() + (Math.max(0, remaining) / Math.max(0.01, speed)) * 1000);
  return `${pad(end.getHours())}:${pad(end.getMinutes())}`;
}

export function fileName(path: string): string {
  if (/^[a-z]+:\/\//i.test(path)) {
    try {
      const u = new URL(path);
      return decodeURIComponent(u.pathname.split('/').filter(Boolean).pop() ?? u.host) || u.host;
    } catch {
      return path;
    }
  }
  return path.split(/[\\/]/).pop() ?? path;
}

export function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length > 1 ? parts[parts.length - 2] : '';
}

/** "Kochelsee.bei.Daemmerung.2160p.mkv" -> "Kochelsee bei Daemmerung". */
export function prettyTitle(name: string): string {
  const base = name.replace(/\.[a-z0-9]{2,4}$/i, '');
  const cut = base.replace(/[._](19|20)\d{2}[._].*$/, '').replace(/[._](2160p|1080p|720p|480p|4k|uhd|bluray|web-?dl|hdr|x26[45]|h\.?26[45]).*$/i, '');
  return cut.replace(/[._]+/g, ' ').trim() || name;
}

export const VIDEO_EXTENSIONS = [
  'mkv', 'mp4', 'm4v', 'mov', 'webm', 'avi', 'wmv', 'flv', 'ts', 'm2ts', 'mts', 'mpg', 'mpeg', 'vob', 'ogv', '3gp', 'mxf', 'divx', 'rm', 'rmvb', 'asf', 'f4v', 'y4m', 'iso',
];
export const AUDIO_EXTENSIONS = ['mp3', 'flac', 'm4a', 'aac', 'ogg', 'opus', 'wav', 'mka', 'ac3', 'dts', 'wma', 'aiff'];
export const SUBTITLE_EXTENSIONS = ['srt', 'ass', 'ssa', 'vtt', 'sub', 'idx', 'sup', 'smi', 'lrc'];

export function extension(path: string): string {
  return (path.split('.').pop() ?? '').toLowerCase();
}
