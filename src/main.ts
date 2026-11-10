import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const app = document.getElementById('app')!;
app.innerHTML = '<div style="position:fixed;left:20px;bottom:20px;padding:10px 16px;border-radius:99px;background:rgba(20,20,26,.5);backdrop-filter:blur(20px);color:#fff;font:14px system-ui" id="t">Spike</div>';
const t = document.getElementById('t')!;
listen<{ kind: string; name?: string; value?: string }>('mpv', (e) => {
  if (e.payload.kind === 'prop' && e.payload.name === 'time-pos') t.textContent = 'time-pos ' + e.payload.value;
  else console.log(JSON.stringify(e.payload));
});
const files = await invoke<string[]>('pending_files');
if (files.length) await invoke('open_files', { files, append: false });
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
document.documentElement.style.boxShadow = 'inset 0 0 0 4px red';
const inner = await getCurrentWindow().innerSize();
console.log('inner', inner.width, inner.height, devicePixelRatio);
t.dataset.size = `${inner.width}x${inner.height}@${devicePixelRatio}`;
setInterval(() => (t.textContent = t.dataset.size + ' ' + t.textContent?.slice(-12)), 500);
