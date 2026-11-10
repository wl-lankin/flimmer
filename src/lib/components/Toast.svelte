<script lang="ts">
  import { ui } from '../ui.svelte';
</script>

{#if ui.toast}
  {#key ui.toast.id}
    <div class="toast glass" role="status">{ui.toast.text}</div>
  {/key}
{/if}

<style>
  .toast {
    position: absolute;
    top: 56px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 500;
    background: var(--glass-strong);
    animation: pop 0.25s var(--ease);
    pointer-events: none;
    z-index: 20;
  }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -6px); } }
</style>
