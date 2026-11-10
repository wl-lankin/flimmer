<script lang="ts">
  import * as mpv from '../mpv';
  import { player } from '../player.svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';

  let intro = $state<{ start: number; end: number } | null>(null);

  $effect(() => {
    const path = player.path;
    intro = null;
    if (!path || /^[a-z]+:\/\//i.test(path)) return;
    let current = true;
    mpv.detectIntro(path).then((found) => current && (intro = found)).catch(() => {});
    return () => (current = false);
  });

  const visible = $derived(intro !== null && player.time >= intro.start && player.time < intro.end - 1.5);
</script>

{#if visible && intro}
  <button class="skip glass" onclick={() => intro && player.seek(intro.end)}>
    {t('skipIntro')}<Icon name="next" size={16} />
  </button>
{/if}

<style>
  .skip {
    position: absolute;
    right: 28px;
    bottom: 96px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 18px;
    border-radius: 12px;
    font-weight: 600;
    font-size: 14px;
    background: rgba(18, 18, 24, 0.7);
    animation: in 0.35s var(--ease);
    z-index: 4;
  }
  .skip:hover { background: var(--beam); color: #1b1206; border-color: var(--beam); }
  @keyframes in { from { opacity: 0; transform: translateX(12px); } }
</style>
