<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { player } from '../player.svelte';
  import { ui } from '../ui.svelte';
  import { clock } from '../format';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';
  import Seekbar from './Seekbar.svelte';
  import VolumeControl from './VolumeControl.svelte';
  import TrackMenu from './TrackMenu.svelte';
  import SpeedMenu from './SpeedMenu.svelte';
  import TogetherPanel from './TogetherPanel.svelte';
  import { together } from '../together.svelte';

  const long = $derived(player.duration >= 3600);
  const toggleMenu = (m: 'tracks' | 'speed' | 'together') => (ui.menu = ui.menu === m ? null : m);
</script>

<div class="capsule glass" role="toolbar" aria-label="Wiedergabe">
  <button class="main" onclick={() => player.togglePause()} aria-label={player.paused ? t('play') : t('pause')}>
    <Icon name={player.eof ? 'replay' : player.paused ? 'play' : 'pause'} size={20} />
  </button>
  <span class="time mono">{clock(player.time, long)}</span>
  <Seekbar />
  <span class="time mono rest">−{clock(player.remaining, long)}</span>
  <VolumeControl />
  <div class="anchor">
    <button class="icon" class:on={ui.menu === 'tracks'} onclick={() => toggleMenu('tracks')} aria-label={t('subtitles')} title={t('subtitles')}>
      <Icon name="subtitles" />
    </button>
    {#if ui.menu === 'tracks'}<TrackMenu />{/if}
  </div>
  <div class="anchor">
    <button class="chip mono" class:on={player.speed !== 1} onclick={() => toggleMenu('speed')} aria-label={t('speed')} title={t('speed')}>
      {player.speed.toLocaleString(undefined, { maximumFractionDigits: 2 })}×
    </button>
    {#if ui.menu === 'speed'}<SpeedMenu />{/if}
  </div>
  <div class="anchor">
    <button class="icon" class:on={!!together.room || ui.menu === 'together'} onclick={() => toggleMenu('together')} aria-label={t('together')} title={t('together')}>
      <Icon name="people" />
    </button>
    {#if ui.menu === 'together'}<TogetherPanel />{/if}
  </div>
  <button class="icon" class:on={ui.studio} onclick={() => { ui.studio = !ui.studio; ui.say(ui.studio ? t('studioOn') : t('studioOff')); }} aria-label={t('studio')} title="{t('studio')} (Tab)">
    <Icon name="studio" />
  </button>
  <button class="icon" onclick={() => getCurrentWindow().setFullscreen(!ui.fullscreen)} aria-label={t('fullscreen')} title="{t('fullscreen')} (F)">
    <Icon name={ui.fullscreen ? 'exitFullscreen' : 'fullscreen'} />
  </button>
</div>

<style>
  .capsule {
    position: absolute;
    left: 50%;
    bottom: 22px;
    transform: translateX(-50%);
    width: min(760px, calc(100% - 32px));
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px 7px 7px;
    border-radius: 999px;
  }
  .main {
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.14);
    transition: background 0.15s, transform 0.15s var(--ease);
  }
  .main:hover { background: rgba(255, 255, 255, 0.24); }
  .main:active { transform: scale(0.94); }
  .time { font-size: 12px; min-width: 38px; color: var(--fg); }
  .rest { color: var(--muted); text-align: right; }
  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
    transition: color 0.15s, background 0.15s;
  }
  .icon:hover { color: var(--fg); background: rgba(255, 255, 255, 0.08); }
  .icon.on, .chip.on { color: var(--beam); }
  .chip { font-size: 12px; padding: 4px 8px; border-radius: 8px; color: var(--muted); }
  .chip:hover { color: var(--fg); background: rgba(255, 255, 255, 0.08); }
  .anchor { position: relative; display: flex; }
  @media (max-width: 620px) {
    .rest, .chip { display: none; }
  }
</style>
