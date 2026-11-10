<script lang="ts" module>
  import { Channel, invoke } from '@tauri-apps/api/core';

  interface Still { index: number; time: number; url: string }
  const COUNT = 14;
  const cache = new Map<string, Still[]>();
  const running = new Set<string>();

  function load(path: string, update: (stills: Still[]) => void) {
    const known = cache.get(path);
    if (known) update([...known]);
    if (running.has(path) || (known && known.length >= COUNT)) return;
    running.add(path);
    const stills: Still[] = known ?? [];
    cache.set(path, stills);
    const channel = new Channel<Still>();
    channel.onmessage = (s) => {
      stills[s.index] = s;
      update([...stills]);
    };
    invoke('filmstrip', { path, count: COUNT, width: 240, channel })
      .catch((e) => console.warn('filmstrip', e))
      .finally(() => running.delete(path));
  }
</script>

<script lang="ts">
  import { player } from '../player.svelte';

  let stills = $state<Still[]>([]);
  $effect(() => {
    const path = player.path;
    stills = [];
    if (path && player.hasVideo) load(path, (s) => (stills = s));
  });
  const current = $derived(player.duration ? Math.min(COUNT - 1, Math.floor((player.time / player.duration) * COUNT)) : -1);
</script>

<div class="strip">
  {#each Array(COUNT) as _, i (i)}
    {@const s = stills[i]}
    <button
      class="cell"
      class:now={i === current}
      style:background-image={s ? `url(${s.url})` : undefined}
      onclick={() => s && player.seek(s.time)}
      aria-label="Still {i + 1}"
      tabindex="-1"
    ></button>
  {/each}
</div>

<style>
  .strip {
    display: grid;
    grid-template-columns: repeat(14, 1fr);
    gap: 3px;
    height: 46px;
  }
  .cell {
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05) center / cover no-repeat;
    opacity: 0.62;
    transition: opacity 0.2s, outline-color 0.2s;
    outline: 2px solid transparent;
    outline-offset: -2px;
    animation: shimmer 1.4s ease-in-out infinite alternate;
  }
  .cell[style*='background-image'] { animation: none; }
  .cell:hover { opacity: 1; }
  .cell.now { opacity: 1; outline-color: var(--beam); }
  @keyframes shimmer { to { background-color: rgba(255, 255, 255, 0.1); } }
  @media (max-width: 700px) {
    .strip { grid-template-columns: repeat(7, 1fr); }
    .cell:nth-child(even) { display: none; }
  }
</style>
