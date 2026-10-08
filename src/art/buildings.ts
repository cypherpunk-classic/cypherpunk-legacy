import { building, neonSign, type ObjectArt } from './objects';
import { fill as rect, prng, px } from './draw';
import { drawText } from './font';
import { COLORS as C } from './palette';

/** District buildings, signs and a few larger set pieces. */

export const BUILDINGS: Record<string, ObjectArt> = {
  exchange: building({ w: 160, h: 84, wall: '#2a3f66', wallDark: '#1d2c4a', roof: '#1a1c2c', trim: '#ef7d57', window: '#ffb347', windowOff: '#141a2e', door: '#3b5dc9', seed: 11, pattern: 'panels', roofStyle: 'sawtooth' }),
  mint: building({ w: 160, h: 88, wall: '#6b5320', wallDark: '#4a3914', roof: '#2b1f0c', trim: '#ffcd75', window: '#ffe7a8', windowOff: '#241a0a', door: '#9c7a2e', seed: 12, pattern: 'bricks', roofStyle: 'peak' }),
  masks: building({ w: 160, h: 84, wall: '#3a2a5c', wallDark: '#2a1e44', roof: '#1d1430', trim: '#ff3e9a', window: '#c792ff', windowOff: '#1a1230', door: '#7d4bd6', seed: 13, pattern: 'panels', roofStyle: 'peak' }),
  observatory: building({ w: 176, h: 96, wall: '#1d2236', wallDark: '#151a2b', roof: '#94b0c2', trim: '#41a6f6', window: '#73eff7', windowOff: '#10162a', door: '#29366f', seed: 14, roofStyle: 'dome' }),
  boathouse: building({ w: 128, h: 72, wall: '#5c3a21', wallDark: '#3e2614', roof: '#2b1810', trim: '#73eff7', window: '#ffcd75', windowOff: '#1f140b', door: '#7a4a28', seed: 15, pattern: 'wood', roofStyle: 'peak' }),
  radioshack: building({ w: 80, h: 60, wall: '#3d4554', wallDark: '#2b313d', roof: '#1f2330', trim: '#ffcd75', window: '#a7f070', windowOff: '#141a14', door: '#566c86', seed: 16, pattern: 'panels' }),
  pavilion: building({ w: 96, h: 56, wall: '#1f4a33', wallDark: '#173326', roof: '#2a5e3c', trim: '#a7f070', window: '#d8ffb0', windowOff: '#0f2418', door: '#2f6b44', seed: 17, pattern: 'wood', roofStyle: 'peak' }),
  tower: building({ w: 96, h: 120, wall: '#262a36', wallDark: '#1a1d26', roof: '#12141c', trim: '#ff5566', window: '#ff8a8a', windowOff: '#14151c', door: '#3d4554', seed: 18, pattern: 'panels' }),
  block_a: building({ w: 112, h: 92, wall: '#2a2440', wallDark: '#1d1930', roof: '#141020', trim: '#73eff7', window: '#73eff7', windowOff: '#141026', door: '#3b5dc9', seed: 19, pattern: 'plain' }),
  block_b: building({ w: 112, h: 92, wall: '#2b3a3a', wallDark: '#1d2828', roof: '#121a1a', trim: '#a7f070', window: '#ffcd75', windowOff: '#121c1c', door: '#257179', seed: 20, pattern: 'bricks' }),
};

export const SIGNS: Record<string, ObjectArt> = {
  wharf: neonSign('THE WHARF', '#73eff7'),
  exchange: neonSign('EXCHANGE', '#ef7d57'),
  commons: neonSign('THE COMMONS', '#a7f070'),
  mint: neonSign('THE MINT', '#ffcd75'),
  masks: neonSign('MASQUERADE', '#ff3e9a'),
  observatory: neonSign('OBSERVATORY', '#73b7ff'),
  plaza: neonSign('LEGACY PLAZA', '#73eff7'),
  boathouse: neonSign('BOATHOUSE', '#ffcd75'),
  radio: neonSign('RADIO', '#a7f070'),
  dla: neonSign('DLA', '#ff5566'),
  qsafe: neonSign('QSAFE', '#ff5566'),
};

const door: ObjectArt = {
  w: 16,
  h: 30,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 16, 30, C.j!);
    rect(g, 2, 2, 12, 28, C.l!);
    for (let i = 0; i < 3; i++) rect(g, 2 + i * 4 + 1, 2, 1, 28, C.k!);
    rect(g, 2, 10, 12, 1, C.k!);
    rect(g, 2, 20, 12, 1, C.k!);
    px(g, 11, 16, C.e!);
    px(g, 11, 17, '#b8862e');
  },
};

const gatepad: ObjectArt = {
  w: 40,
  h: 20,
  frames: 3,
  draw(g, f) {
    for (let y = 0; y < 20; y++)
      for (let x = 0; x < 40; x++) {
        const dx = (x + 0.5 - 20) / 20;
        const dy = (y + 0.5 - 10) / 10;
        const d = dx * dx + dy * dy;
        if (d > 1) continue;
        const ring = Math.abs(Math.sqrt(d) - (0.35 + f * 0.25)) < 0.08;
        px(g, x, y, ring ? '#73eff7' : d > 0.85 ? '#3ef0ff' : d > 0.6 ? '#1f6b78' : '#12303a');
      }
  },
};

const archive: ObjectArt = {
  w: 32,
  h: 40,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 32, 40, '#3d4554');
    rect(g, 2, 2, 28, 36, f ? '#0b0d16' : '#5f6b7d');
    if (!f) {
      for (let y = 6; y < 36; y += 6) rect(g, 4, y, 24, 1, '#4a5263');
      rect(g, 10, 14, 12, 12, '#b13e53');
      rect(g, 11, 15, 10, 10, '#7a2638');
      drawText(g, 'DLA', 11, 18, '#ffcdd2', 1);
      rect(g, 24, 18, 3, 6, '#94b0c2');
    } else {
      rect(g, 6, 8, 20, 26, '#0f2a33');
      for (let i = 0; i < 5; i++) rect(g, 8, 10 + i * 5, 16, 3, ['#73eff7', '#ffcd75', '#a7f070', '#c792ff', '#ef7d57'][i]!);
    }
  },
};

const checkpoint: ObjectArt = {
  w: 48,
  h: 34,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 6, 8, 28, '#262a36');
    rect(g, 40, 6, 8, 28, '#262a36');
    rect(g, 0, 0, 48, 8, '#1f2330');
    drawText(g, 'CHECKPOINT', 5, 2, '#ff5566', 0);
    rect(g, 2, 12, 4, 6, f ? '#ff5566' : '#7a2638');
    rect(g, 42, 12, 4, 6, f ? '#5dff9a' : '#1f5a3a');
    rect(g, 8, 20, 32, 3, f ? '#ff3e9a' : '#b13e78');
    for (let x = 10; x < 40; x += 6) rect(g, x, 20, 3, 3, '#ffcdd2');
  },
};

const billboard: ObjectArt = {
  w: 72,
  h: 44,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 10, 30, 3, 14, '#3d4554');
    rect(g, 59, 30, 3, 14, '#3d4554');
    rect(g, 0, 0, 72, 32, '#1a1c2c');
    rect(g, 2, 2, 68, 28, f ? '#2a0f18' : '#3a121f');
    drawText(g, 'LAWFUL ACCESS', 9, 6, '#ffcdd2', 1);
    drawText(g, 'KEEPS YOU', 18, 14, '#ff5566', 1);
    drawText(g, 'SAFE', 28, 22, '#ff5566', 1);
    if (f) rect(g, 2, 17, 68, 1, '#ff556644');
  },
};

const shard: ObjectArt = {
  w: 12,
  h: 16,
  frames: 2,
  outline: true,
  draw(g, f) {
    const col = f ? '#a4ffe0' : '#73eff7';
    for (let y = 0; y < 16; y++) {
      const w = y < 8 ? Math.floor(y * 0.75) + 1 : Math.floor((15 - y) * 0.75) + 1;
      rect(g, 6 - w, y, w * 2, 1, y % 4 === 0 ? '#ffffff' : col);
    }
  },
};

const radio: ObjectArt = {
  w: 26,
  h: 20,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 4, 26, 16, '#566c86');
    rect(g, 0, 4, 26, 2, '#94b0c2');
    rect(g, 3, 8, 10, 8, '#1a1c2c');
    for (let i = 0; i < 4; i++) rect(g, 4, 9 + i * 2, 8, 1, '#3d4554');
    rect(g, 16, 8, 7, 3, f ? '#a7f070' : '#38b764');
    px(g, 17, 14, '#ffcd75');
    px(g, 20, 14, '#ffcd75');
    rect(g, 20, 0, 1, 5, '#94b0c2');
  },
};

const whiteboard: ObjectArt = {
  w: 40,
  h: 30,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 40, 24, '#c7d3dc');
    rect(g, 2, 2, 36, 20, '#f4f4f4');
    const r = prng(5);
    for (let i = 0; i < 6; i++) rect(g, 4 + Math.floor(r() * 4), 4 + i * 3, 10 + Math.floor(r() * 18), 1, i % 2 ? '#3b5dc9' : '#b13e53');
    rect(g, 6, 24, 2, 6, '#5f6b7d');
    rect(g, 32, 24, 2, 6, '#5f6b7d');
  },
};

export const EXTRA_OBJECTS: Record<string, ObjectArt> = { door, gatepad, archive, checkpoint, billboard, shard, radio, whiteboard };
