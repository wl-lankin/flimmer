<script lang="ts">
  import { player } from '../player.svelte';
  import * as mpv from '../mpv';
  import { glow, videoRect, type Rgb } from '../ambient';

  let { margin = 0 }: { margin?: number } = $props();

  let width = $state(window.innerWidth);
  let height = $state(window.innerHeight);
  let colors = $state<Rgb[]>([[30, 30, 40], [30, 30, 40], [30, 30, 40], [30, 30, 40]]);

  const rect = $derived(videoRect(width, height, player.aspect, margin));
  const css = (c: Rgb, a = 0.75) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  const spread = $derived(Math.round(Math.min(width, height) * 0.09));
  const shadow = $derived(
    [
      `${-spread}px 0 ${spread * 2}px ${spread / 3}px ${css(colors[0])}`,
      `${spread}px 0 ${spread * 2}px ${spread / 3}px ${css(colors[1])}`,
      `0 ${-spread}px ${spread * 2}px ${spread / 3}px ${css(colors[2], 0.6)}`,
      `0 ${spread}px ${spread * 2}px ${spread / 3}px ${css(colors[3], 0.6)}`,
      `0 0 0 200vmax var(--room)`,
    ].join(', '),
  );

  let busy = false;
  async function sample() {
    if (busy || !player.hasVideo) return;
    busy = true;
    try {
      const c = await mpv.edgeColors();
      colors = c.map((x) => glow(x as Rgb));
    } catch {
      /* no frame yet */
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    const resize = () => {
      width = window.innerWidth;
      height = window.innerHeight;
    };
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  });

  $effect(() => {
    void player.seekCount;
    void player.path;
    const t = setTimeout(sample, 250);
    if (player.paused) return () => clearTimeout(t);
    const id = setInterval(sample, 700);
    return () => {
      clearTimeout(t);
      clearInterval(id);
    };
  });
</script>

<div
  class="hole"
  class:inset={margin > 0}
  style:left="{rect.x}px"
  style:top="{rect.y}px"
  style:width="{rect.w}px"
  style:height="{rect.h}px"
  style:box-shadow={shadow}
  aria-hidden="true"
></div>

<style>
  .hole {
    position: absolute;
    pointer-events: none;
    transition: box-shadow 1.1s ease;
  }
  .inset { border-radius: 12px; }
</style>
