<script lang="ts">
  import * as mpv from '../mpv';
  import { player } from '../player.svelte';
  import { clock, fileName, prettyTitle } from '../format';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';

  let { onclose }: { onclose: () => void } = $props();
  let query = $state('');
  let hits = $state<mpv.Hit[]>([]);
  let searched = $state(false);

  $effect(() => {
    const q = query;
    if (q.trim().length < 2) {
      hits = [];
      searched = false;
      return;
    }
    const id = setTimeout(async () => {
      hits = await mpv.transcriptSearch(q).catch(() => []);
      searched = true;
    }, 180);
    return () => clearTimeout(id);
  });

  async function go(h: mpv.Hit) {
    if (h.path === player.path) await player.seek(Math.max(0, h.time - 1));
    else await mpv.command('loadfile', h.path, 'replace', -1, `start=${Math.max(0, h.time - 1)}`);
    onclose();
  }

  function mark(text: string): [string, string, string] {
    const i = text.toLowerCase().indexOf(query.trim().toLowerCase());
    if (i < 0) return [text, '', ''];
    const n = query.trim().length;
    return [text.slice(0, i), text.slice(i, i + n), text.slice(i + n)];
  }
</script>

<div class="scrim" role="presentation" onclick={onclose}>
  <div class="panel glass" role="dialog" aria-label={t('search')} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && onclose()}>
    <label class="field">
      <Icon name="subtitles" />
      <!-- svelte-ignore a11y_autofocus -->
      <input id="search" bind:value={query} placeholder={t('searchPlaceholder')} autofocus spellcheck="false" aria-label={t('search')} />
    </label>
    {#if hits.length}
      <ul>
        {#each hits as h, i (i)}
          {@const [a, b, c] = mark(h.text)}
          <li>
            <button onclick={() => go(h)}>
              <span class="time mono">{clock(h.time)}</span>
              <span class="text">{a}<mark>{b}</mark>{c}</span>
              {#if h.path !== player.path}<span class="file">{prettyTitle(fileName(h.path))}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="hint">{searched ? t('searchNone') : t('searchEmpty')}</p>
    {/if}
  </div>
</div>

<style>
  .scrim { position: absolute; inset: 0; background: rgba(6, 6, 10, 0.45); z-index: 40; display: flex; justify-content: center; align-items: flex-start; padding-top: 12vh; }
  .panel {
    width: min(640px, calc(100% - 32px));
    border-radius: 16px;
    background: var(--glass-strong);
    overflow: hidden;
    animation: drop 0.22s var(--ease);
  }
  .field { display: flex; align-items: center; gap: 12px; padding: 14px 18px; border-bottom: 1px solid var(--line); color: var(--muted); }
  input { flex: 1; background: none; border: 0; outline: 0; color: var(--fg); font: 500 17px var(--f-text); user-select: text; }
  ul { list-style: none; margin: 0; padding: 6px; max-height: 50vh; overflow: auto; }
  li button { width: 100%; display: grid; grid-template-columns: 64px 1fr; gap: 2px 12px; padding: 9px 12px; border-radius: 10px; text-align: left; }
  li button:hover { background: rgba(255, 255, 255, 0.07); }
  .time { color: var(--beam); font-size: 12px; padding-top: 2px; }
  .text { line-height: 1.4; }
  .file { grid-column: 2; color: var(--muted); font-size: 12px; }
  mark { background: var(--beam-soft); color: var(--beam); border-radius: 3px; padding: 0 2px; }
  .hint { margin: 0; padding: 16px 18px 18px; color: var(--muted); }
  @keyframes drop { from { opacity: 0; transform: translateY(-8px); } }
</style>
