import { getCurrentWindow } from '@tauri-apps/api/window';
import { readText } from './clipboard';
import * as mpv from './mpv';
import { player } from './player.svelte';
import { ui } from './ui.svelte';
import { t, language } from './i18n';
import { trackName } from './tracks';

const speedLabel = (s: number) => `${s.toLocaleString(undefined, { maximumFractionDigits: 2 })}×`;

/** Global shortcuts. Returns true when the key was used. */
export async function handleKey(e: KeyboardEvent, openDialog: () => void): Promise<boolean> {
  const target = e.target as HTMLElement | null;
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return false;
  const mod = e.metaKey || e.ctrlKey;
  const k = e.key;

  if (mod && k.toLowerCase() === 'o') return openDialog(), true;
  if (mod && k.toLowerCase() === 'v') {
    const text = (await readText())?.trim();
    if (text) await mpv.openFiles([text]);
    return true;
  }
  if (mod) return false;
  if (k === 'Escape') {
    if (ui.menu) ui.menu = null;
    else if (ui.fullscreen) await getCurrentWindow().setFullscreen(false);
    return true;
  }
  if (k === 'f' || k === 'F') return getCurrentWindow().setFullscreen(!ui.fullscreen), true;
  if (k === 'Tab') {
    ui.studio = !ui.studio;
    ui.say(ui.studio ? t('studioOn') : t('studioOff'));
    return true;
  }
  if (player.idle) return false;

  switch (k) {
    case ' ':
    case 'k':
    case 'K':
      await player.togglePause();
      return true;
    case 'ArrowLeft':
    case 'ArrowRight': {
      const step = e.shiftKey ? 30 : e.altKey ? 1 : 5;
      await player.seekBy(k === 'ArrowLeft' ? -step : step);
      return true;
    }
    case 'ArrowUp':
    case 'ArrowDown': {
      const v = player.volume + (k === 'ArrowUp' ? 5 : -5);
      await player.setVolume(v);
      ui.rememberVolume(player.volume);
      ui.say(`${t('volume')} ${Math.round(player.volume)} %`);
      return true;
    }
    case 'm':
    case 'M':
      await player.toggleMute();
      ui.say(player.muted ? `${t('volume')} ${Math.round(player.volume)} %` : t('muted'));
      return true;
    case ',':
      await player.frameStep(false);
      return true;
    case '.':
      await player.frameStep(true);
      return true;
    case '[':
    case ']':
      await player.setSpeed(player.speed + (k === '[' ? -0.25 : 0.25));
      ui.say(speedLabel(player.speed));
      return true;
    case 'Backspace':
      await player.setSpeed(1);
      ui.say(speedLabel(1));
      return true;
    case 's':
    case 'S':
      await player.screenshot();
      ui.say(t('screenshot'));
      return true;
    case 'c':
    case 'C':
      await player.cycleTrack('sub');
      setTimeout(() => ui.say(`${t('subtitles')}: ${player.currentSub ? language(player.currentSub.lang) || trackName(player.currentSub) : t('off')}`), 120);
      return true;
    case 'a':
    case 'A':
      await player.cycleTrack('audio');
      setTimeout(() => player.currentAudio && ui.say(`${t('audio')}: ${trackName(player.currentAudio)}`), 120);
      return true;
    case 'l':
    case 'L':
      await player.cycleLoop();
      setTimeout(() => ui.say(player.loopA === null ? t('loopOff') : player.loopB === null ? t('loopA') : t('loopAB')), 120);
      return true;
    case 'i':
    case 'I':
      ui.info = !ui.info;
      return true;
    case 'g':
    case 'G':
      ui.aura = !ui.aura;
      ui.say(ui.aura ? t('auraOn') : t('auraOff'));
      return true;
    case 'PageUp':
    case 'PageDown':
      await player.nextChapter(k === 'PageUp' ? -1 : 1);
      return true;
    case 'n':
    case 'N':
      await mpv.command('playlist-next');
      return true;
    case 'p':
    case 'P':
      await mpv.command('playlist-prev');
      return true;
  }
  if (/^[0-9]$/.test(k) && player.duration) {
    await player.seek((Number(k) / 10) * player.duration);
    return true;
  }
  return false;
}
