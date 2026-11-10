<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isWindows } from '../ui.svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';

  let { title = '', visible = true }: { title?: string; visible?: boolean } = $props();
  const w = getCurrentWindow();
</script>

<header class="bar" class:hidden={!visible} data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>{title}</span>
  {#if isWindows}
    <div class="buttons">
      <button onclick={() => w.minimize()} aria-label={t('minimize')}><Icon name="minimize" size={16} /></button>
      <button onclick={() => w.toggleMaximize()} aria-label={t('maximize')}><Icon name="maximize" size={14} /></button>
      <button class="close" onclick={() => w.close()} aria-label={t('close')}><Icon name="close" size={16} /></button>
    </div>
  {/if}
</header>

<style>
  .bar {
    position: absolute;
    inset: 0 0 auto 0;
    height: 52px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 9px;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.55), transparent);
    transition: opacity 0.35s var(--ease);
    z-index: 5;
  }
  .hidden { opacity: 0; }
  .title {
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 8px rgba(0, 0, 0, 0.5);
  }
  .buttons { position: absolute; top: 0; right: 0; display: flex; }
  .buttons button { width: 46px; height: 34px; display: grid; place-items: center; color: rgba(255, 255, 255, 0.8); }
  .buttons button:hover { background: rgba(255, 255, 255, 0.1); }
  .buttons .close:hover { background: #c42b1c; color: #fff; }
</style>
