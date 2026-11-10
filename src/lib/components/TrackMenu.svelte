<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { player } from '../player.svelte';
  import { t } from '../i18n';
  import { trackName, trackNote } from '../tracks';
  import { SUBTITLE_EXTENSIONS } from '../format';
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';

  async function loadSubtitle() {
    const file = await open({ multiple: false, filters: [{ name: t('subtitles'), extensions: SUBTITLE_EXTENSIONS }] });
    if (typeof file === 'string') await player.addSubtitle(file);
  }
  const delay = (d: number) => player.setSubDelay(Math.round((player.subDelay + d) * 10) / 10);
</script>

<Popover align="right">
  {#if player.audioTracks.length}
    <h4>{t('audio')}</h4>
    {#each player.audioTracks as tr (tr.id)}
      <button class="item" role="menuitemradio" aria-checked={!!tr.selected} onclick={() => player.selectTrack('audio', tr.id)}>
        <span class="mark">{#if tr.selected}<Icon name="check" size={14} />{/if}</span>
        {trackName(tr)}<small>{trackNote(tr)}</small>
      </button>
    {/each}
  {/if}
  <h4>{t('subtitles')}</h4>
  <button class="item" role="menuitemradio" aria-checked={!player.currentSub} onclick={() => player.selectTrack('sub', 'no')}>
    <span class="mark">{#if !player.currentSub}<Icon name="check" size={14} />{/if}</span>{t('off')}
  </button>
  {#each player.subTracks as tr (tr.id)}
    <button class="item" role="menuitemradio" aria-checked={!!tr.selected} onclick={() => player.selectTrack('sub', tr.id)}>
      <span class="mark">{#if tr.selected}<Icon name="check" size={14} />{/if}</span>
      {trackName(tr)}<small>{trackNote(tr)}</small>
    </button>
  {/each}
  <button class="item" onclick={loadSubtitle}><span class="mark"></span>{t('loadSubtitle')}</button>
  {#if player.currentSub}
    <div class="delay">
      <span>{t('delay')}</span>
      <button onclick={() => delay(-0.1)} aria-label="−0,1 s">−</button>
      <span class="mono">{player.subDelay >= 0 ? '+' : ''}{player.subDelay.toFixed(1)} s</span>
      <button onclick={() => delay(0.1)} aria-label="+0,1 s">+</button>
    </div>
  {/if}
  {#if player.videoTracks.length > 1}
    <h4>{t('video')}</h4>
    {#each player.videoTracks as tr (tr.id)}
      <button class="item" role="menuitemradio" aria-checked={!!tr.selected} onclick={() => player.selectTrack('video', tr.id)}>
        <span class="mark">{#if tr.selected}<Icon name="check" size={14} />{/if}</span>
        {trackName(tr)}<small>{tr['demux-w']}×{tr['demux-h']}</small>
      </button>
    {/each}
  {/if}
</Popover>

<style>
  .delay { display: flex; align-items: center; gap: 8px; padding: 6px 10px; font-size: 13px; }
  .delay > span:first-child { color: var(--muted); margin-right: auto; }
  .delay button { width: 26px; height: 26px; border-radius: 7px; background: rgba(255, 255, 255, 0.08); }
  .delay button:hover { background: rgba(255, 255, 255, 0.16); }
  .mono { min-width: 54px; text-align: center; font-size: 12px; }
</style>
