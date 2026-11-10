<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import * as mpv from './lib/mpv';
  import { player } from './lib/player.svelte';
  import { ui, isMac } from './lib/ui.svelte';
  import { handleKey } from './lib/keys';
  import { SUBTITLE_EXTENSIONS, extension, fileName, prettyTitle } from './lib/format';
  import { lang, t } from './lib/i18n';
  import Welcome from './lib/components/Welcome.svelte';
  import Capsule from './lib/components/Capsule.svelte';
  import StudioBar from './lib/components/StudioBar.svelte';
  import PausePoster from './lib/components/PausePoster.svelte';
  import InfoHud from './lib/components/InfoHud.svelte';
  import Aura from './lib/components/Aura.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Toast from './lib/components/Toast.svelte';
  import SkipIntro from './lib/components/SkipIntro.svelte';
  import SubsProgress from './lib/components/SubsProgress.svelte';
  import Search from './lib/components/Search.svelte';
  import { subs } from './lib/subs.svelte';

  const AURA_MARGIN = 0.035;
  const HIDE_AFTER = 2200;

  let welcome: Welcome | undefined = $state();
  let searching = $state(false);
  let overControls = false;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const showUi = $derived(player.idle || ui.controls || player.paused || ui.menu !== null || player.eof);
  const auraOn = $derived(ui.aura && player.hasVideo && !player.idle);
  const margin = $derived(auraOn && !ui.fullscreen ? AURA_MARGIN : 0);
  const title = $derived(player.idle ? '' : player.title && player.title !== fileName(player.path) ? player.title : prettyTitle(fileName(player.path)));

  let posterReady = $state(false);
  $effect(() => {
    if (!(player.paused || player.eof) || player.idle) {
      posterReady = false;
      return;
    }
    const id = setTimeout(() => (posterReady = true), 450);
    return () => clearTimeout(id);
  });
  const poster = $derived(!player.idle && ((posterReady && player.hasVideo) || (!player.hasVideo && player.duration > 0)));

  function poke() {
    ui.controls = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (!overControls && !ui.menu) ui.controls = false;
    }, HIDE_AFTER);
  }

  $effect(() => {
    const visible = showUi;
    if (isMac && !ui.fullscreen) mpv.windowButtons(visible).catch(() => {});
  });

  $effect(() => {
    const m = String(margin);
    for (const side of ['left', 'right', 'top', 'bottom']) mpv.set(`video-margin-ratio-${side}`, m).catch(() => {});
  });

  $effect(() => {
    if (player.lastError) ui.say(`${t('error')}: ${player.lastError}`);
  });

  async function open(paths: string[]) {
    if (!paths.length) return;
    if (!player.idle && paths.every((p) => SUBTITLE_EXTENSIONS.includes(extension(p)))) {
      for (const p of paths) await player.addSubtitle(p);
      return;
    }
    await mpv.openFiles(paths);
  }

  let clickTimer: ReturnType<typeof setTimeout> | undefined;
  function stageClick(e: MouseEvent) {
    if (ui.menu) {
      ui.menu = null;
      return;
    }
    if (e.detail > 1) return;
    clickTimer = setTimeout(() => player.togglePause(), 200);
  }
  function stageDouble() {
    clearTimeout(clickTimer);
    getCurrentWindow().setFullscreen(!ui.fullscreen);
  }

  onMount(() => {
    const cleanups: (() => void)[] = [];
    const w = getCurrentWindow();
    mpv.onEvent((e) => {
      player.handle(e);
      if (e.kind === 'file-loaded') setTimeout(() => player.path && subs.attach(player.path), 300);
    }).then((u) => cleanups.push(u));
    listen<string[]>('open-files', (e) => open(e.payload)).then((u) => cleanups.push(u));
    w.onResized(async () => (ui.fullscreen = await w.isFullscreen())).then((u) => cleanups.push(u));
    getCurrentWebview()
      .onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === 'enter') ui.dropping = p.paths.length && p.paths.every((x) => SUBTITLE_EXTENSIONS.includes(extension(x))) && !player.idle ? 'subtitle' : 'video';
        else if (p.type === 'leave') ui.dropping = null;
        else if (p.type === 'drop') {
          ui.dropping = null;
          open(p.paths);
        }
      })
      .then((u) => cleanups.push(u));

    Promise.all([
      mpv.set('volume', ui.savedVolume),
      mpv.set('alang', lang === 'de' ? 'de,ger,deu,en,eng' : 'en,eng'),
      mpv.set('slang', lang === 'de' ? 'de,ger,deu' : 'en,eng'),
      mpv.set('subs-with-matching-audio', 'forced'),
    ])
      .catch(() => {})
      .then(() => mpv.pendingFiles())
      .then(open);

    const key = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        searching = !searching;
        return;
      }
      if (searching) return;
      handleKey(e, () => welcome?.pickFiles() ?? pickFromPlayer()).then((used) => used && poke());
      if (['Tab', ' ', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'PageUp', 'PageDown'].includes(e.key)) e.preventDefault();
    };
    const menu = (e: MouseEvent) => import.meta.env.PROD && e.preventDefault();
    window.addEventListener('keydown', key);
    window.addEventListener('contextmenu', menu);
    cleanups.push(() => window.removeEventListener('keydown', key), () => window.removeEventListener('contextmenu', menu));
    return () => cleanups.forEach((c) => c());
  });

  async function pickFromPlayer() {
    const { open: dialog } = await import('@tauri-apps/plugin-dialog');
    const files = await dialog({ multiple: true });
    if (files?.length) await open(files);
  }
</script>

<div
  class="app"
  class:nocursor={!showUi}
  role="presentation"
  onmousemove={poke}
  onmouseleave={() => !player.paused && (ui.controls = false)}
>
  {#if player.idle}
    <Welcome bind:this={welcome} />
  {:else}
    {#if !player.hasVideo}<div class="room"></div>{/if}
    {#if auraOn}<Aura {margin} />{/if}
    <div class="stage" role="presentation" onclick={stageClick} ondblclick={stageDouble}></div>
    {#if poster}<PausePoster />{/if}
    {#if ui.info && player.hasVideo && showUi}<InfoHud />{/if}
    <div
      class="controls"
      class:hidden={!showUi}
      role="presentation"
      onmouseenter={() => (overControls = true)}
      onmouseleave={() => (overControls = false)}
    >
      {#if ui.studio}<StudioBar />{:else}<Capsule />{/if}
    </div>
    <SkipIntro />
    {#if player.buffering}<div class="buffering" aria-label="Lädt"></div>{/if}
  {/if}
  <TitleBar {title} visible={showUi && !player.idle} />
  <SubsProgress />
  <Toast />
  {#if searching}<Search onclose={() => (searching = false)} />{/if}
  {#if ui.dropping}
    <div class="drop">
      <div class="drop-card">{ui.dropping === 'subtitle' ? t('dropSubtitle') : t('dropRelease')}</div>
    </div>
  {/if}
</div>

<style>
  .app { position: fixed; inset: 0; overflow: hidden; }
  .nocursor, .nocursor * { cursor: none !important; }
  .room { position: absolute; inset: 0; background: radial-gradient(120% 90% at 80% 0%, #1a1620, var(--room)); }
  .stage { position: absolute; inset: 0; }
  .controls { transition: opacity 0.35s var(--ease), transform 0.35s var(--ease); }
  .controls.hidden { opacity: 0; transform: translateY(8px); pointer-events: none; }
  .buffering {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 42px;
    height: 42px;
    margin: -21px;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.18);
    border-top-color: var(--beam);
    animation: spin 0.9s linear infinite;
    pointer-events: none;
  }
  .drop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(10, 11, 16, 0.55);
    backdrop-filter: blur(6px);
    z-index: 30;
    pointer-events: none;
  }
  .drop-card {
    padding: 22px 34px;
    border-radius: 18px;
    border: 2px dashed var(--beam);
    color: var(--beam);
    font: 600 18px var(--f-display);
    background: rgba(10, 11, 16, 0.6);
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
