import { describe, expect, it } from 'vitest';
import { trackName, trackNote } from './tracks';

describe('tracks', () => {
  it('prefers the language and keeps a distinct title', () => {
    expect(trackName({ id: 1, type: 'audio', lang: 'eng', title: 'Commentary' })).toMatch(/·  ?Commentary|· Commentary/);
    expect(trackName({ id: 2, type: 'sub' })).toMatch(/2$/);
  });
  it('notes codec and channels', () => {
    expect(trackNote({ id: 1, type: 'audio', codec: 'eac3', 'demux-channel-count': 6 })).toBe('EAC3 · 5.1');
    expect(trackNote({ id: 3, type: 'sub', codec: 'subrip' })).toBe('SRT');
  });
});
