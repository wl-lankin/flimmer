<script lang="ts">
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { together } from '../together.svelte';
  import { ui } from '../ui.svelte';
  import { t } from '../i18n';
  import Popover from './Popover.svelte';

  let code = $state('');
  let name = $state(together.name);

  async function copy() {
    if (!together.room) return;
    await writeText(together.room).catch(() => {});
    ui.say(t('copied'));
  }
  function start(action: 'create' | 'join') {
    together.rememberName(name);
    together.open(action, code);
  }
</script>

<Popover align="right">
  <div class="panel">
    <h4>{t('together')}</h4>
    {#if together.room}
      <button class="code mono" onclick={copy} title={t('copied')}>{together.room}</button>
      <p class="hint">{t('inRoom')}</p>
      <ul>
        {#each together.peers as p (p.id)}
          <li class:me={p.id === together.me}><span class="dot"></span>{p.name}</li>
        {/each}
      </ul>
      <button class="secondary" onclick={() => together.leave()}>{t('leave')}</button>
    {:else}
      <p class="hint">{t('togetherIntro')}</p>
      <label for="together-name">{t('yourName')}</label>
      <input id="together-name" bind:value={name} maxlength="32" spellcheck="false" />
      <button class="primary" disabled={together.connecting} onclick={() => start('create')}>{t('createRoom')}</button>
      <div class="join">
        <input id="together-code" bind:value={code} maxlength="6" placeholder={t('roomCode')} class="mono" spellcheck="false" aria-label={t('roomCode')} />
        <button class="secondary" disabled={together.connecting || code.trim().length !== 6} onclick={() => start('join')}>{t('join')}</button>
      </div>
    {/if}
  </div>
</Popover>

<style>
  .panel { width: 270px; display: flex; flex-direction: column; gap: 8px; padding: 2px 4px 4px; }
  h4 { margin-left: 0 !important; }
  .hint { margin: 0; color: var(--muted); font-size: 12.5px; line-height: 1.45; }
  label { font-size: 12px; color: var(--muted); margin-top: 4px; }
  input {
    width: 100%;
    min-width: 0;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--line);
    border-radius: 9px;
    padding: 8px 10px;
    color: var(--fg);
    font: inherit;
    user-select: text;
    outline: 0;
  }
  input:focus { border-color: var(--beam); }
  .join { display: flex; gap: 6px; }
  .join input { text-transform: uppercase; letter-spacing: 0.2em; }
  .primary, .secondary { padding: 9px 12px; border-radius: 9px; font-weight: 600; font-size: 13px; }
  .primary { background: var(--beam); color: #1b1206; }
  .secondary { border: 1px solid var(--line); }
  .secondary:hover { background: rgba(255, 255, 255, 0.06); }
  button:disabled { opacity: 0.5; cursor: default; }
  .code {
    font-size: 30px;
    letter-spacing: 0.22em;
    color: var(--beam);
    padding: 8px;
    border-radius: 10px;
    background: var(--beam-soft);
    text-align: center;
  }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  li { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  li.me { color: var(--muted); }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: #6fd68f; }
</style>
