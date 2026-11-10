import { Channel, invoke } from '@tauri-apps/api/core';
import * as mpv from './mpv';
import { player } from './player.svelte';
import { ui } from './ui.svelte';
import { lang, language, t } from './i18n';

type Progress =
  | { kind: 'model'; done: number; total: number }
  | { kind: 'listening'; percent: number; srt: string | null; lang: string }
  | { kind: 'translating'; percent: number }
  | { kind: 'done'; srt: string; lang: string; translated: string | null }
  | { kind: 'failed'; message: string };

const title = (code: string, translated = false) =>
  `${t('subsWhisper')} · ${language(code) || code}${translated ? ` (${t('translated')})` : ''}`;

/** Finds the track mpv made for an external subtitle file. */
const trackOf = (file: string) => player.subTracks.find((tr) => tr.external && (tr as { 'external-filename'?: string })['external-filename'] === file);

class Subs {
  status = $state<string | null>(null);
  percent = $state(0);
  running = $state(false);

  /** Adds subtitles Flimmer made earlier for this file, without selecting them. */
  async attach(path: string) {
    const known = await mpv.transcriptFor(path).catch(() => null);
    if (!known || player.path !== path) return;
    await mpv.command('sub-add', known.srt, 'auto', title(known.lang), known.lang).catch(() => {});
    if (known.translated) await mpv.command('sub-add', known.translated, 'auto', title(lang, true), lang).catch(() => {});
  }

  async create() {
    if (this.running) return ui.say(t('subsBusy'));
    const path = player.path;
    if (!path || /^[a-z]+:\/\//i.test(path)) return;
    this.running = true;
    this.percent = 0;
    this.status = t('subsListening');
    let added = false;
    const channel = new Channel<Progress>();
    channel.onmessage = async (p) => {
      const here = player.path === path;
      switch (p.kind) {
        case 'model':
          this.status = `${t('subsModel')} · ${Math.round(p.done / 1e6)} / ${Math.round(p.total / 1e6)} MB`;
          this.percent = p.total ? (p.done / p.total) * 100 : 0;
          break;
        case 'listening':
          this.status = t('subsListening');
          this.percent = p.percent;
          if (here && p.srt) {
            const track = trackOf(p.srt);
            if (!added) {
              added = true;
              await mpv.command('sub-add', p.srt, 'select', title(p.lang), p.lang).catch(() => {});
            } else if (track) {
              await mpv.command('sub-reload', track.id).catch(() => {});
            }
          }
          break;
        case 'translating':
          this.status = t('subsTranslating');
          this.percent = p.percent;
          break;
        case 'done':
          this.running = false;
          this.status = null;
          if (here) {
            const track = trackOf(p.srt);
            if (track) await mpv.command('sub-reload', track.id).catch(() => {});
            else await mpv.command('sub-add', p.srt, 'select', title(p.lang), p.lang).catch(() => {});
            if (p.translated) await mpv.command('sub-add', p.translated, 'select', title(lang, true), lang).catch(() => {});
          }
          ui.say(t('subsDone'));
          break;
        case 'failed':
          this.running = false;
          this.status = null;
          if (p.message !== 'cancelled') ui.say(`${t('subsFailed')}: ${p.message}`);
          break;
      }
    };
    invoke('subtitles_create', { path, duration: player.duration, lang, channel }).catch((e) => {
      this.running = false;
      this.status = null;
      ui.say(`${t('subsFailed')}: ${e}`);
    });
  }

  cancel() {
    mpv.subtitlesCancel();
  }
}

export const subs = new Subs();
