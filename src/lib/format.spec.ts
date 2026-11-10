import { describe, expect, it } from 'vitest';
import { clock, endsAt, fileName, humanDuration, prettyTitle, timecode } from './format';

describe('format', () => {
  it('clock switches to hours for long media', () => {
    expect(clock(72)).toBe('1:12');
    expect(clock(2472)).toBe('41:12');
    expect(clock(3723)).toBe('1:02:03');
    expect(clock(30, true)).toBe('0:00:30');
    expect(clock(NaN)).toBe('0:00');
  });

  it('timecode counts frames', () => {
    expect(timecode(2472.34, 23.976)).toBe('00:41:12:08');
    expect(timecode(0, 25)).toBe('00:00:00:00');
  });

  it('humanDuration reads like speech', () => {
    expect(humanDuration(4256, 'de')).toBe('1 Std. 11 Min.');
    expect(humanDuration(3600, 'de')).toBe('1 Std.');
    expect(humanDuration(20, 'de')).toBe('unter 1 Min.');
    expect(humanDuration(600, 'en')).toBe('10 min');
  });

  it('endsAt honours the speed', () => {
    const now = new Date(2026, 10, 10, 21, 30, 0);
    expect(endsAt(4260, 1, now)).toBe('22:41');
    expect(endsAt(3600, 2, now)).toBe('22:00');
  });

  it('names files and urls', () => {
    expect(fileName('/Users/me/Filme/a b.mkv')).toBe('a b.mkv');
    expect(fileName('C:\\Filme\\x.mp4')).toBe('x.mp4');
    expect(fileName('https://example.com/videos/Clip%201.mp4')).toBe('Clip 1.mp4');
  });

  it('prettyTitle drops release noise', () => {
    expect(prettyTitle('Kochelsee.bei.Daemmerung.2160p.HDR.mkv')).toBe('Kochelsee bei Daemmerung');
    expect(prettyTitle('Der_Film.2019.German.mkv')).toBe('Der Film');
    expect(prettyTitle('urlaub.mp4')).toBe('urlaub');
  });
});
