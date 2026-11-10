function load<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem('flimmer.' + key);
    return v === null ? fallback : (JSON.parse(v) as T);
  } catch {
    return fallback;
  }
}
function save(key: string, value: unknown) {
  try {
    localStorage.setItem('flimmer.' + key, JSON.stringify(value));
  } catch {
    /* storage may be unavailable; preferences then last one session */
  }
}

export type Menu = 'tracks' | 'speed' | 'together' | null;

class Ui {
  controls = $state(true);
  menu = $state<Menu>(null);
  fullscreen = $state(false);
  toast = $state<{ text: string; id: number } | null>(null);
  dropping = $state<'video' | 'subtitle' | null>(null);
  #studio = $state(load('studio', false));
  #aura = $state(load('aura', true));
  #info = $state(load('info', false));
  #volume = load('volume', 100);

  get studio() { return this.#studio; }
  set studio(v: boolean) { this.#studio = v; save('studio', v); }
  get aura() { return this.#aura; }
  set aura(v: boolean) { this.#aura = v; save('aura', v); }
  get info() { return this.#info; }
  set info(v: boolean) { this.#info = v; save('info', v); }
  get savedVolume() { return this.#volume; }
  rememberVolume(v: number) { this.#volume = v; save('volume', v); }

  #toastTimer: ReturnType<typeof setTimeout> | undefined;
  say(text: string) {
    this.toast = { text, id: Date.now() };
    clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => (this.toast = null), 1300);
  }
}

export const ui = new Ui();
export const isWindows = navigator.userAgent.includes('Windows');
export const isMac = navigator.userAgent.includes('Mac');
