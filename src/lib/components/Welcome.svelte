<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import * as mpv from '../mpv';
  import { AUDIO_EXTENSIONS, VIDEO_EXTENSIONS, clock, fileName, folderName, prettyTitle } from '../format';
  import { t } from '../i18n';
  import { isMac } from '../ui.svelte';
  import Icon from './Icon.svelte';

  let recent = $state<{ path: string; start: number; saved: number }[]>([]);
  let url = $state('');
  let urlOpen = $state(false);

  $effect(() => {
    mpv.recentFiles().then((r) => (recent = r)).catch(() => {});
  });

  export async function pickFiles() {
    const files = await open({
      multiple: true,
      filters: [
        { name: 'Video', extensions: [...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS] },
        { name: '*', extensions: ['*'] },
      ],
    });
    if (files?.length) await mpv.openFiles(files);
  }
  async function playUrl(e: SubmitEvent) {
    e.preventDefault();
    const u = url.trim();
    if (u) await mpv.openFiles([u]);
  }
  const mod = isMac ? '⌘' : 'Strg+';
</script>

<main class="welcome">
  <div class="beam" aria-hidden="true"></div>
  <div class="grain" aria-hidden="true"></div>
  <section class="hero">
    <h1 class="word" aria-label="Flimmer">
      {#each 'Flimmer'.split('') as ch, i (i)}<span style:--i={i}>{ch}</span>{/each}
    </h1>
    <p class="tag">{t('tagline')}</p>
    <div class="actions">
      <button class="primary" onclick={pickFiles}><Icon name="folder" />{t('open')}<kbd>{mod}O</kbd></button>
      {#if urlOpen}
        <form class="url" onsubmit={playUrl}>
          <Icon name="link" />
          <!-- svelte-ignore a11y_autofocus -->
          <input id="url" bind:value={url} placeholder={t('urlPlaceholder')} autofocus spellcheck="false" />
          <button type="submit">{t('play')}</button>
        </form>
      {:else}
        <button class="ghost" onclick={() => (urlOpen = true)}><Icon name="link" />{t('openUrl')}</button>
      {/if}
    </div>
    <p class="drop">{t('dropHere')}</p>
  </section>

  <section class="recent" aria-label={t('continue')}>
    <h2>{t('continue')}</h2>
    {#if recent.length}
      <ul>
        {#each recent as r (r.path)}
          <li>
            <button onclick={() => mpv.openFiles([r.path])} title={r.path}>
              <span class="name">{prettyTitle(fileName(r.path))}</span>
              <span class="meta">{folderName(r.path)}</span>
              <span class="pos mono">{t('at')} {clock(r.start)}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">{t('noRecent')}</p>
    {/if}
  </section>
</main>

<style>
  .welcome {
    position: absolute;
    inset: 0;
    background: var(--room);
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: 6vw;
    align-items: center;
    padding: 64px max(32px, 6vw) 40px;
    overflow: hidden;
  }
  .beam {
    position: absolute;
    inset: -30% -20% auto auto;
    width: 90vw;
    height: 110vh;
    background:
      radial-gradient(closest-side, rgba(255, 180, 84, 0.2), transparent 70%),
      conic-gradient(from 200deg at 85% 10%, transparent 0deg, rgba(255, 196, 120, 0.07) 18deg, transparent 40deg);
    pointer-events: none;
    animation: drift 14s ease-in-out infinite alternate;
    mask-image: radial-gradient(closest-side, #000 35%, transparent 100%);
  }
  .grain {
    position: absolute;
    inset: 0;
    opacity: 0.05;
    background-image: repeating-radial-gradient(circle at 17% 32%, #fff 0 0.6px, transparent 0.6px 3px);
    pointer-events: none;
  }
  .hero, .recent { position: relative; min-width: 0; }
  .word {
    margin: 0;
    font: 800 clamp(64px, 12vw, 168px) / 0.88 var(--f-display);
    letter-spacing: -0.05em;
    display: flex;
  }
  .word span { animation: flicker 7s steps(1) infinite; animation-delay: calc(var(--i) * -0.9s); }
  .word span:nth-child(3) { color: var(--beam); }
  .tag { margin: 18px 0 30px; font-size: clamp(15px, 1.6vw, 19px); color: var(--muted); }
  .actions { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; }
  .primary, .ghost, .url {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    height: 44px;
    padding: 0 18px;
    border-radius: 12px;
    font-weight: 600;
  }
  .primary { background: var(--beam); color: #1b1206; box-shadow: 0 10px 40px -12px var(--beam); }
  .primary:hover { filter: brightness(1.06); }
  .primary kbd { font: 500 11px var(--f-data); opacity: 0.6; margin-left: 4px; }
  .ghost { border: 1px solid var(--line); color: var(--fg); }
  .ghost:hover { background: rgba(255, 255, 255, 0.05); }
  .url { border: 1px solid var(--beam); padding: 0 6px 0 14px; width: min(440px, 100%); font-weight: 400; }
  .url input { flex: 1; min-width: 0; background: none; border: 0; outline: 0; color: var(--fg); font: inherit; user-select: text; }
  .url button { height: 32px; padding: 0 12px; border-radius: 8px; background: var(--beam); color: #1b1206; font-weight: 600; }
  .drop { margin-top: 26px; font: 500 11px var(--f-data); letter-spacing: 0.14em; text-transform: uppercase; color: var(--muted); }
  .recent h2 {
    margin: 0 0 12px;
    font: 500 11px var(--f-data);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--beam);
  }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; max-height: 58vh; overflow: auto; }
  li button {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 14px;
    padding: 11px 12px;
    border-radius: 10px;
    text-align: left;
    border-top: 1px solid var(--line);
  }
  li:first-child button { border-top-color: transparent; }
  li button:hover { background: rgba(255, 255, 255, 0.05); border-top-color: transparent; }
  .name { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { grid-row: 2; color: var(--muted); font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pos { grid-row: 1 / span 2; align-self: center; font-size: 12px; color: var(--muted); }
  .empty { color: var(--muted); max-width: 36ch; margin: 0; }
  @keyframes flicker {
    0%, 100% { opacity: 1; }
    92% { opacity: 1; }
    93% { opacity: 0.55; }
    94% { opacity: 1; }
    96% { opacity: 0.8; }
  }
  @keyframes drift { to { transform: translate(-4vw, 3vh) rotate(-4deg); } }
  @media (max-width: 760px) {
    .welcome { grid-template-columns: 1fr; align-content: center; gap: 36px; }
    ul { max-height: 30vh; }
  }
</style>
