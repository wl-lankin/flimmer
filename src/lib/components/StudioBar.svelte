<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { player } from '../player.svelte';
  import { ui } from '../ui.svelte';
  import { clock, timecode } from '../format';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';
  import Seekbar from './Seekbar.svelte';
  import Filmstrip from './Filmstrip.svelte';
  import VolumeControl from './VolumeControl.svelte';
  import TrackMenu from './TrackMenu.svelte';
  import SpeedMenu from './SpeedMenu.svelte';

  const fps = $derived(player.tracks.find((tr) => tr.type === 'video' && tr.selected)?.['demux-fps'] ?? player.fps ?? 0);
  const tc = $derived(timecode(player.time, fps));
  const loopLabel = $derived(player.loopA === null ? 'A-B' : player.loopB === null ? 'A-…' : 'A-B ✓');
  const toggleMenu = (m: 'tracks' | 'speed') => (ui.menu = ui.menu === m ? null : m);
</script>

<div class="studio glass" role="toolbar" aria-label="Studio">
  <Filmstrip />
  <Seekbar tall />
  <div class="row">
    <span class="tc mono">{tc.slice(0, 8)}<span class="frames">:{tc.slice(9)}</span></span>
    <span class="dur mono">/ {clock(player.duration, true)}</span>
    <div class="transport">
      <button class="icon" onclick={() => player.frameStep(false)} title="{t('frameBack')} (,)" aria-label={t('frameBack')}><Icon name="frameBack" /></button>
      <button class="main" onclick={() => player.togglePause()} aria-label={player.paused ? t('play') : t('pause')}>
        <Icon name={player.eof ? 'replay' : player.paused ? 'play' : 'pause'} size={20} />
      </button>
      <button class="icon" onclick={() => player.frameStep(true)} title="{t('frameNext')} (.)" aria-label={t('frameNext')}><Icon name="frameNext" /></button>
    </div>
    <span class="spacer"></span>
    <button class="chip mono" class:on={player.loopA !== null} onclick={() => player.cycleLoop()} title="L">{loopLabel}</button>
    <div class="anchor">
      <button class="chip mono" class:on={player.speed !== 1} onclick={() => toggleMenu('speed')}>{player.speed.toLocaleString(undefined, { maximumFractionDigits: 2 })}×</button>
      {#if ui.menu === 'speed'}<SpeedMenu />{/if}
    </div>
    {#if player.chapters.length}
      <span class="chip mono">{t('chapter')} {player.chapter + 1}/{player.chapters.length}</span>
    {/if}
    <div class="anchor">
      <button class="icon" class:on={ui.menu === 'tracks'} onclick={() => toggleMenu('tracks')} aria-label={t('subtitles')}><Icon name="subtitles" /></button>
      {#if ui.menu === 'tracks'}<TrackMenu />{/if}
    </div>
    <VolumeControl />
    <button class="icon" onclick={() => player.screenshot().then(() => ui.say(t('screenshot')))} title="{t('screenshot')} (S)" aria-label={t('screenshot')}><Icon name="camera" /></button>
    <button class="icon" class:on={ui.info} onclick={() => (ui.info = !ui.info)} title="{t('info')} (I)" aria-label={t('info')}><Icon name="info" /></button>
    <button class="icon on" onclick={() => { ui.studio = false; ui.say(t('studioOff')); }} title="{t('studio')} (Tab)" aria-label={t('studio')}><Icon name="studio" /></button>
    <button class="icon" onclick={() => getCurrentWindow().setFullscreen(!ui.fullscreen)} aria-label={t('fullscreen')}><Icon name={ui.fullscreen ? 'exitFullscreen' : 'fullscreen'} /></button>
  </div>
</div>

<style>
  .studio {
    position: absolute;
    left: 14px;
    right: 14px;
    bottom: 14px;
    padding: 10px 14px 10px;
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: rgba(13, 14, 19, 0.78);
  }
  .row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .tc { font-size: 17px; color: var(--fg); }
  .frames { color: var(--muted); }
  .dur { font-size: 12px; color: var(--muted); }
  .transport { display: flex; align-items: center; gap: 4px; margin-left: 10px; }
  .spacer { flex: 1; }
  .main {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.14);
  }
  .main:hover { background: rgba(255, 255, 255, 0.24); }
  .icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .icon:hover { color: var(--fg); background: rgba(255, 255, 255, 0.08); }
  .icon.on, .chip.on { color: var(--beam); }
  .chip {
    font-size: 11.5px;
    padding: 4px 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    color: var(--muted);
  }
  button.chip:hover { color: var(--fg); }
  .chip.on { border-color: color-mix(in srgb, var(--beam) 40%, transparent); }
  .anchor { position: relative; display: flex; }
</style>
