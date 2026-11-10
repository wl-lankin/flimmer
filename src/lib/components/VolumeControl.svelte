<script lang="ts">
  import { player } from '../player.svelte';
  import { ui } from '../ui.svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';

  let open = $state(false);
  const icon = $derived(player.muted || player.volume === 0 ? 'mute' : player.volume < 50 ? 'volumeLow' : 'volume');

  function input(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    player.setVolume(v);
    ui.rememberVolume(v);
    if (player.muted && v > 0) player.toggleMute();
  }
</script>

<div class="vol" class:open role="group" aria-label={t('volume')} onmouseenter={() => (open = true)} onmouseleave={() => (open = false)}>
  <button class="icon" onclick={() => player.toggleMute()} aria-label={t('muted')} title="{t('volume')} (M)">
    <Icon name={icon} />
  </button>
  <input
    id="volume"
    type="range"
    min="0"
    max="130"
    step="1"
    value={player.muted ? 0 : player.volume}
    oninput={input}
    aria-label={t('volume')}
    style:--fill="{((player.muted ? 0 : player.volume) / 130) * 100}%"
  />
</div>

<style>
  .vol { display: flex; align-items: center; }
  .icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .icon:hover { color: var(--fg); background: rgba(255, 255, 255, 0.08); }
  input {
    appearance: none;
    -webkit-appearance: none;
    width: 0;
    opacity: 0;
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(90deg, #fff var(--fill), rgba(255, 255, 255, 0.2) var(--fill));
    transition: width 0.22s var(--ease), opacity 0.22s, margin 0.22s;
    cursor: pointer;
  }
  .open input, input:focus-visible { width: 84px; opacity: 1; margin: 0 6px 0 2px; }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
  }
</style>
