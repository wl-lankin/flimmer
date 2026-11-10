import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

class Updater {
  available = $state<Update | null>(null);
  installing = $state(false);

  /** Looks for a newer release once; quietly does nothing offline or in development. */
  async look() {
    if (import.meta.env.DEV) return;
    try {
      this.available = await check();
    } catch (e) {
      console.warn('update check', e);
    }
  }

  async install() {
    if (!this.available || this.installing) return;
    this.installing = true;
    try {
      await this.available.downloadAndInstall();
      await relaunch();
    } catch (e) {
      console.warn('update install', e);
      this.installing = false;
    }
  }
}

export const updater = new Updater();
