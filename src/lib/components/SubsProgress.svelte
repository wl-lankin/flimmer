<script lang="ts">
  import { subs } from '../subs.svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';
</script>

{#if subs.status}
  <div class="job glass" role="status">
    <span class="ring" style:--p={subs.percent}></span>
    <span class="text">{subs.status}</span>
    <span class="mono pct">{Math.round(subs.percent)} %</span>
    <button onclick={() => subs.cancel()} aria-label={t('cancel')} title={t('cancel')}><Icon name="close" size={14} /></button>
  </div>
{/if}

<style>
  .job {
    position: absolute;
    top: 46px;
    left: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px 7px 10px;
    border-radius: 999px;
    font-size: 12.5px;
    background: var(--glass-strong);
    z-index: 6;
    animation: in 0.3s var(--ease);
  }
  .ring {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: conic-gradient(var(--beam) calc(var(--p) * 1%), rgba(255, 255, 255, 0.15) 0);
    mask: radial-gradient(circle, transparent 4.5px, #000 5px);
  }
  .pct { color: var(--muted); font-size: 11.5px; }
  button { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; color: var(--muted); }
  button:hover { background: rgba(255, 255, 255, 0.1); color: var(--fg); }
  @keyframes in { from { opacity: 0; transform: translateY(-6px); } }
</style>
