<script lang="ts">
  import { player } from '../player.svelte';
  import { t } from '../i18n';

  const v = $derived(player.video);
  const vt = $derived(player.tracks.find((tr) => tr.type === 'video' && tr.selected));
  const at = $derived(player.currentAudio);
  const fps = $derived(vt?.['demux-fps'] ?? player.fps);
  const range = $derived(
    v.gamma === 'pq' ? `HDR10 (PQ)${v['sig-peak'] ? ` · ${Math.round((v['sig-peak'] ?? 0) * 203)} nits` : ''}` : v.gamma === 'hlg' ? 'HLG' : 'SDR',
  );
  const lines = $derived([
    [`${(player.videoCodec || vt?.codec || '').toUpperCase()}${vt?.['codec-profile'] ? ' ' + vt['codec-profile'] : ''}`, `${v.w ?? '?'}×${v.h ?? '?'}`],
    [`${fps ? fps.toFixed(3).replace(/\.?0+$/, '') : '?'} fps`, `${range} · ${v.primaries ?? ''}`],
    [`${(player.audioCodec || at?.codec || '').toUpperCase()}`, at?.['demux-channels'] ?? ''],
    [t('decoder'), player.hwdec && player.hwdec !== 'no' ? player.hwdec : t('software')],
    [t('buffer'), `${Math.max(0, player.cacheTime - player.time).toFixed(1)} s`],
  ]);
</script>

<div class="hud glass mono" aria-label={t('info')}>
  {#each lines as [a, b], i (i)}
    <div><span>{a}</span><b>{b}</b></div>
  {/each}
</div>

<style>
  .hud {
    position: absolute;
    top: 44px;
    right: 16px;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(8, 8, 12, 0.66);
    font-size: 11px;
    line-height: 1.7;
    display: flex;
    flex-direction: column;
    min-width: 230px;
    pointer-events: none;
  }
  div > div { display: flex; justify-content: space-between; gap: 18px; }
  span { color: var(--muted); }
  b { font-weight: 500; color: var(--beam); }
</style>
