<script lang="ts">
  import { player } from '../player.svelte';
  import { t } from '../i18n';
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';

  const speeds = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 3];
</script>

<Popover>
  <h4>{t('speed')}</h4>
  {#each speeds as s (s)}
    <button class="item" role="menuitemradio" aria-checked={player.speed === s} onclick={() => player.setSpeed(s)}>
      <span class="mark">{#if player.speed === s}<Icon name="check" size={14} />{/if}</span>
      <span class="mono">{s.toLocaleString(undefined, { minimumFractionDigits: s % 1 ? 1 : 0 })}×</span>
    </button>
  {/each}
</Popover>
