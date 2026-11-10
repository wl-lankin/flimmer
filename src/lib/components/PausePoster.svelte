<script lang="ts">
  import { player } from '../player.svelte';
  import { endsAt, fileName, humanDuration, prettyTitle } from '../format';
  import { lang, language, t } from '../i18n';
  import { trackName } from '../tracks';

  const title = $derived.by(() => {
    const name = player.title || fileName(player.path);
    return name === fileName(player.path) ? prettyTitle(name) : name;
  });
  const eyebrow = $derived(
    [
      player.eof ? t('ended') : player.paused ? t('paused') : t('audio'),
      player.chapters.length && player.chapter >= 0 ? `${t('chapter')} ${player.chapter + 1}` : '',
      player.currentChapter?.title ?? '',
    ]
      .filter(Boolean)
      .join(' · '),
  );
  const details = $derived(
    [
      player.eof ? '' : `${t('left')} ${humanDuration(player.remaining, lang)}`,
      player.currentSub ? `${t('subtitles')} ${language(player.currentSub.lang) || trackName(player.currentSub)}` : '',
      player.currentAudio && player.audioTracks.length > 1 ? `${t('audio')} ${trackName(player.currentAudio)}` : '',
    ]
      .filter(Boolean)
      .join(' · '),
  );
  let now = $state(new Date());
  $effect(() => {
    const id = setInterval(() => (now = new Date()), 15000);
    return () => clearInterval(id);
  });
</script>

<div class="shade"></div>
<div class="poster">
  <span class="eyebrow">{eyebrow}</span>
  <h1>{title}</h1>
  {#if details}<p>{details}</p>{/if}
  {#if !player.eof && player.duration}
    <span class="ends mono">{t('endsAt')} {endsAt(player.remaining, player.speed, now)}</span>
  {/if}
</div>

<style>
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, rgba(6, 6, 10, 0.86) 0%, rgba(6, 6, 10, 0.55) 42%, rgba(6, 6, 10, 0) 78%);
    animation: fade 0.5s var(--ease);
    pointer-events: none;
  }
  .poster {
    position: absolute;
    left: max(36px, 6vw);
    top: 50%;
    transform: translateY(-58%);
    max-width: min(62%, 760px);
    display: flex;
    flex-direction: column;
    gap: clamp(6px, 1.4vh, 14px);
    pointer-events: none;
    animation: enter 0.6s var(--ease);
  }
  .eyebrow {
    font: 500 clamp(10px, 1.1vw, 12.5px) var(--f-data);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--beam);
  }
  h1 {
    margin: 0;
    font: 800 clamp(30px, 6.2vw, 92px) / 0.95 var(--f-display);
    letter-spacing: -0.035em;
    text-wrap: balance;
    overflow-wrap: anywhere;
  }
  p { margin: 0; font-size: clamp(12px, 1.3vw, 16px); color: rgba(255, 255, 255, 0.76); }
  .ends { font-size: clamp(12px, 1.3vw, 15px); color: var(--beam); }
  @keyframes fade { from { opacity: 0; } }
  @keyframes enter { from { opacity: 0; transform: translate(-14px, -58%); } }
</style>
