<script lang="ts">
  import { player } from '../player.svelte';
  import { clock } from '../format';

  let { tall = false }: { tall?: boolean } = $props();

  let bar: HTMLDivElement;
  let hover = $state<number | null>(null);
  let dragging = $state(false);

  const fraction = $derived(player.duration > 0 ? player.time / player.duration : 0);
  const cached = $derived(player.duration > 0 ? Math.min(1, player.cacheTime / player.duration) : 0);
  const hoverChapter = $derived.by(() => {
    if (hover === null) return undefined;
    const at = hover * player.duration;
    let found;
    for (const c of player.chapters) if (c.time <= at) found = c;
    return found;
  });

  function at(e: PointerEvent): number {
    const r = bar.getBoundingClientRect();
    return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  }
  function down(e: PointerEvent) {
    if (!player.duration) return;
    bar.setPointerCapture(e.pointerId);
    dragging = true;
    player.seek(at(e) * player.duration, false);
  }
  function move(e: PointerEvent) {
    hover = at(e);
    if (dragging) player.seek(hover * player.duration, false);
  }
  function up(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    player.seek(at(e) * player.duration, true);
  }
</script>

<div
  class="seek"
  class:tall
  class:active={dragging}
  bind:this={bar}
  role="slider"
  tabindex="-1"
  aria-label="Position"
  aria-valuemin={0}
  aria-valuemax={Math.round(player.duration)}
  aria-valuenow={Math.round(player.time)}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointerleave={() => !dragging && (hover = null)}
>
  <div class="rail">
    <div class="cache" style:width="{cached * 100}%"></div>
    <div class="done" style:width="{fraction * 100}%"></div>
    {#if player.loopA !== null && player.duration}
      <div
        class="loop"
        style:left="{(player.loopA / player.duration) * 100}%"
        style:width="{(((player.loopB ?? player.time) - player.loopA) / player.duration) * 100}%"
      ></div>
    {/if}
    {#each player.chapters as c, i (i)}
      {#if i > 0 && player.duration}
        <span class="tick" style:left="{(c.time / player.duration) * 100}%"></span>
      {/if}
    {/each}
  </div>
  <div class="knob" style:left="{fraction * 100}%"></div>
  {#if hover !== null && player.duration}
    <div class="tip glass" style:left="clamp(40px, {hover * 100}%, calc(100% - 40px))">
      {#if hoverChapter?.title}<span class="ch">{hoverChapter.title}</span>{/if}
      <span class="mono">{clock(hover * player.duration, player.duration >= 3600)}</span>
    </div>
  {/if}
</div>

<style>
  .seek {
    position: relative;
    flex: 1;
    min-width: 60px;
    height: 22px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }
  .rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.18);
    overflow: hidden;
    transition: height 0.18s var(--ease);
  }
  .seek:hover .rail, .seek.active .rail, .tall .rail { height: 6px; }
  .cache { position: absolute; inset: 0 auto 0 0; background: rgba(255, 255, 255, 0.16); }
  .done { position: absolute; inset: 0 auto 0 0; background: #fff; }
  .tall .done { background: var(--beam); }
  .loop { position: absolute; top: 0; bottom: 0; background: color-mix(in srgb, var(--beam) 55%, transparent); }
  .tick { position: absolute; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: rgba(10, 11, 16, 0.75); }
  .knob {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.5);
    transform: scale(0);
    transition: transform 0.15s var(--ease);
  }
  .seek:hover .knob, .seek.active .knob { transform: scale(1); }
  .tip {
    position: absolute;
    bottom: 26px;
    transform: translateX(-50%);
    padding: 5px 9px;
    border-radius: 8px;
    font-size: 12px;
    white-space: nowrap;
    display: flex;
    gap: 8px;
    pointer-events: none;
    background: var(--glass-strong);
  }
  .ch { color: var(--muted); max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
</style>
