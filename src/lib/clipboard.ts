import { readText as read } from '@tauri-apps/plugin-clipboard-manager';

/** Clipboard text, or null when there is none or the system refuses. */
export async function readText(): Promise<string | null> {
  try {
    return await read();
  } catch {
    return null;
  }
}
