<script lang="ts">
  import type { Snippet } from 'svelte';
  let { children, align = 'center' }: { children: Snippet; align?: 'center' | 'right' } = $props();
</script>

<div class="pop glass" class:right={align === 'right'} role="menu" tabindex="-1" onpointerdown={(e) => e.stopPropagation()}>
  {@render children()}
</div>

<style>
  .pop {
    position: absolute;
    bottom: calc(100% + 14px);
    left: 50%;
    transform: translateX(-50%);
    min-width: 220px;
    max-height: min(60vh, 440px);
    overflow: auto;
    padding: 8px;
    border-radius: 14px;
    background: var(--glass-strong);
    animation: rise 0.18s var(--ease);
    z-index: 10;
  }
  .right { left: auto; right: -8px; transform: none; }
  @keyframes rise { from { opacity: 0; translate: 0 6px; } }
  .pop :global(h4) {
    margin: 8px 10px 4px;
    font: 500 10.5px var(--f-data);
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .pop :global(.item) {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 8px;
    text-align: left;
    font-size: 13px;
  }
  .pop :global(.item:hover) { background: rgba(255, 255, 255, 0.08); }
  .pop :global(.item .mark) { width: 16px; color: var(--beam); display: grid; place-items: center; }
  .pop :global(.item small) { color: var(--muted); margin-left: auto; font-family: var(--f-data); font-size: 11px; }
  .pop :global(hr) { border: 0; border-top: 1px solid var(--line); margin: 6px 4px; }
</style>
