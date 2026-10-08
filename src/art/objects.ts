import { COLORS as C } from './palette';
import { addOutline, ctx2d, fill, makeCanvas, paint, prng, px, type Rows } from './draw';
import { drawText, textWidth } from './font';

/**
 * Props and furniture. Each entry paints one or more frames; BootScene turns
 * them into spritesheets named `obj-<key>`.
 */

export interface ObjectArt {
  w: number;
  h: number;
  frames?: number;
  outline?: boolean;
  draw: (g: CanvasRenderingContext2D, frame: number) => void;
}

const rect = fill;

// ---------------------------------------------------------------------------
// Cabin
// ---------------------------------------------------------------------------

const bed: ObjectArt = {
  w: 28,
  h: 34,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 28, 6, C.k!); // headboard
    rect(g, 1, 1, 26, 3, C.l!);
    rect(g, 1, 6, 26, 27, C.n!); // sheet
    rect(g, 4, 7, 9, 6, C['6']!); // pillows
    rect(g, 15, 7, 9, 6, C['6']!);
    rect(g, 4, 12, 9, 1, C['5']!);
    rect(g, 15, 12, 9, 1, C['5']!);
    rect(g, 1, 15, 26, 18, '#7a2e45'); // quilt
    for (let y = 15; y < 33; y += 4)
      for (let x = 1 + ((y / 4) % 2) * 3; x < 27; x += 6) rect(g, x, y, 3, 2, '#9c3d5a');
    rect(g, 1, 15, 26, 1, '#b8546f');
    rect(g, 0, 33, 28, 1, C.k!);
  },
};

const bookshelf: ObjectArt = {
  w: 30,
  h: 34,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 30, 34, C.k!);
    rect(g, 2, 2, 26, 30, C.j!);
    const r = prng(7);
    const spines = [C.g!, C.b!, C.e!, '#3b5dc9', C.c!, C.f!, '#7d4bd6', C.n!];
    for (let shelf = 0; shelf < 3; shelf++) {
      const y = 3 + shelf * 10;
      rect(g, 2, y + 8, 26, 2, C.l!);
      let x = 3;
      while (x < 26) {
        const w = 2 + Math.floor(r() * 2);
        const h = 5 + Math.floor(r() * 3);
        if (r() < 0.12) {
          x += 2;
          continue;
        }
        rect(g, x, y + 8 - h, w, h, spines[Math.floor(r() * spines.length)]!);
        px(g, x, y + 8 - h, '#ffffff33');
        x += w;
      }
    }
    rect(g, 0, 32, 30, 2, C.j!);
  },
};

const deskCrt: ObjectArt = {
  w: 32,
  h: 30,
  frames: 2,
  outline: true,
  draw(g, f) {
    // desk
    rect(g, 0, 14, 32, 10, C.l!);
    rect(g, 0, 14, 32, 2, C.m!);
    rect(g, 1, 24, 3, 6, C.k!);
    rect(g, 28, 24, 3, 6, C.k!);
    rect(g, 4, 24, 24, 2, C.k!);
    // CRT
    rect(g, 6, 0, 20, 16, '#cfc6a8');
    rect(g, 6, 13, 20, 3, '#b3a98a');
    rect(g, 8, 2, 16, 10, '#101a14');
    if (f === 1) {
      rect(g, 9, 3, 9, 1, '#3cff8a');
      rect(g, 9, 5, 12, 1, '#2bbf66');
      rect(g, 9, 7, 7, 1, '#2bbf66');
      rect(g, 9, 9, 2, 1, '#3cff8a');
    } else {
      rect(g, 9, 3, 2, 1, '#1d3a29');
    }
    px(g, 23, 13, f === 1 ? '#5dff9a' : '#7a2e1e');
    // keyboard
    rect(g, 9, 17, 14, 4, '#d8d0b3');
    for (let i = 10; i < 22; i += 2) rect(g, i, 18, 1, 2, '#9c9480');
    // sticky note
    rect(g, 24, 9, 5, 5, C.z!);
    rect(g, 25, 10, 3, 1, '#b8a94a');
    rect(g, 25, 12, 2, 1, '#b8a94a');
  },
};

const dresser: ObjectArt = {
  w: 22,
  h: 24,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 22, 22, C.l!);
    rect(g, 0, 0, 22, 2, C.m!);
    for (let i = 0; i < 3; i++) {
      const y = 4 + i * 6;
      rect(g, 2, y, 18, 5, C.k!);
      rect(g, 3, y + 1, 16, 3, C.m!);
      px(g, 11, y + 2, C.e!);
    }
    if (f === 1) {
      rect(g, 2, 4, 18, 7, C.j!);
      rect(g, 4, 5, 9, 4, C.o!);
      rect(g, 5, 6, 6, 1, '#8a7a60');
      rect(g, 5, 8, 4, 1, '#8a7a60');
    }
    rect(g, 1, 22, 3, 2, C.k!);
    rect(g, 18, 22, 3, 2, C.k!);
  },
};

const fireplace: ObjectArt = {
  w: 32,
  h: 34,
  frames: 3,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 32, 34, '#4b4f5c');
    const r = prng(3);
    for (let y = 0; y < 34; y += 4)
      for (let x = (y / 4) % 2 ? 0 : 3; x < 32; x += 7) {
        rect(g, x, y, 6, 3, r() < 0.5 ? '#5f6b7d' : '#565e6e');
      }
    rect(g, 0, 10, 32, 3, C.l!); // mantel
    rect(g, 0, 10, 32, 1, C.m!);
    rect(g, 6, 15, 20, 17, '#140d0b');
    rect(g, 7, 28, 18, 3, C.k!); // logs
    rect(g, 9, 27, 14, 2, C.l!);
    const flames = [
      [10, 18, 4, 10],
      [14, 16, 5, 12],
      [19, 19, 3, 9],
    ];
    flames.forEach(([x, y, w, h], i) => {
      const dy = (f + i) % 3;
      rect(g, x!, y! + dy, w!, h! - dy, C.y!);
      rect(g, x! + 1, y! + 3 + dy, w! - 2, h! - 4 - dy, C.z!);
    });
    px(g, 15, 14 + f, C.x!);
    // candles + photo on mantel
    rect(g, 3, 6, 2, 4, C.o!);
    px(g, 3, 5, C.z!);
    rect(g, 24, 4, 6, 6, C.k!);
    rect(g, 25, 5, 4, 4, '#94b0c2');
  },
};

const plant: ObjectArt = {
  w: 14,
  h: 22,
  outline: true,
  draw(g) {
    rect(g, 3, 14, 8, 8, C.q!);
    rect(g, 3, 14, 8, 2, '#c06a4a');
    const leaves: [number, number, number, number][] = [
      [6, 1, 2, 13],
      [2, 4, 4, 3],
      [8, 3, 4, 3],
      [1, 8, 5, 3],
      [8, 7, 5, 3],
      [4, 11, 6, 3],
    ];
    for (const [x, y, w, h] of leaves) {
      rect(g, x, y, w, h, C.v!);
      px(g, x, y, '#3f8a55');
    }
  },
};

const armchair: ObjectArt = {
  w: 18,
  h: 20,
  outline: true,
  draw(g) {
    rect(g, 1, 0, 16, 10, '#5d275d');
    rect(g, 0, 6, 4, 12, '#4a1f4a');
    rect(g, 14, 6, 4, 12, '#4a1f4a');
    rect(g, 4, 9, 10, 8, '#7a3a7a');
    rect(g, 4, 9, 10, 1, '#9c4f9c');
    rect(g, 1, 18, 2, 2, C.k!);
    rect(g, 15, 18, 2, 2, C.k!);
  },
};

const sideTable: ObjectArt = {
  w: 14,
  h: 24,
  outline: true,
  draw(g) {
    rect(g, 1, 12, 12, 4, C.l!);
    rect(g, 1, 12, 12, 1, C.m!);
    rect(g, 2, 16, 2, 8, C.k!);
    rect(g, 10, 16, 2, 8, C.k!);
    rect(g, 5, 9, 4, 3, '#5f6b7d'); // lamp base
    rect(g, 3, 2, 8, 7, '#e8c39e'); // shade
    rect(g, 3, 8, 8, 1, '#c79c6e');
  },
};

const photo: ObjectArt = {
  w: 12,
  h: 10,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 12, 10, C.m!);
    rect(g, 1, 1, 10, 8, '#2a3f66');
    const heads = ['#e8c39e', '#c98a5d', '#9a6440', '#f6d2b0', '#6b4129'];
    heads.forEach((c, i) => {
      px(g, 2 + i * 2, 4, c);
      px(g, 2 + i * 2, 6, ['#ef7d57', '#38b764', '#ffcd75', '#3b5dc9', '#b13e53'][i]!);
    });
    px(g, 6, 2, '#ffcd75');
  },
};

const rug: ObjectArt = {
  w: 48,
  h: 30,
  draw(g) {
    rect(g, 2, 0, 44, 30, '#5d275d');
    rect(g, 0, 2, 48, 26, '#5d275d');
    rect(g, 3, 3, 42, 24, '#7a2e45');
    rect(g, 6, 6, 36, 18, '#2a2440');
    rect(g, 8, 8, 32, 14, '#7a2e45');
    for (let i = 0; i < 4; i++) {
      rect(g, 14 + i * 6, 13, 3, 3, C.e!);
      px(g, 15 + i * 6, 12, C.f!);
    }
    for (let x = 4; x < 44; x += 4) {
      px(g, x, 0, '#b8546f');
      px(g, x, 29, '#b8546f');
    }
  },
};

const coatRack: ObjectArt = {
  w: 12,
  h: 28,
  outline: true,
  draw(g) {
    rect(g, 5, 2, 2, 24, C.k!);
    rect(g, 2, 25, 8, 2, C.k!);
    rect(g, 1, 2, 10, 2, C.k!);
    rect(g, 1, 4, 5, 10, '#3b5dc9'); // a jacket
    rect(g, 2, 13, 3, 3, '#29366f');
    rect(g, 7, 3, 4, 6, '#b13e53'); // scarf
  },
};

const boxes: ObjectArt = {
  w: 18,
  h: 18,
  outline: true,
  draw(g) {
    rect(g, 0, 6, 12, 12, C.m!);
    rect(g, 0, 6, 12, 2, C.n!);
    rect(g, 5, 6, 2, 12, C.l!);
    rect(g, 7, 0, 11, 10, '#b98a55');
    rect(g, 7, 0, 11, 2, C.n!);
    rect(g, 11, 0, 2, 10, C.l!);
    drawText(g, 'PGP', 8, 4, C.k!, 0);
  },
};

const note: ObjectArt = {
  w: 10,
  h: 8,
  draw(g) {
    rect(g, 0, 0, 10, 8, C.z!);
    rect(g, 1, 2, 7, 1, '#9c8f3a');
    rect(g, 1, 4, 5, 1, '#9c8f3a');
  },
};

// ---------------------------------------------------------------------------
// Cryptopolis
// ---------------------------------------------------------------------------

const lamp: ObjectArt = {
  w: 14,
  h: 40,
  outline: true,
  draw(g) {
    rect(g, 6, 6, 2, 32, '#3d4554');
    rect(g, 4, 37, 6, 3, '#3d4554');
    rect(g, 2, 2, 10, 4, '#3d4554');
    rect(g, 3, 5, 8, 2, C.a!);
    rect(g, 5, 0, 4, 2, '#3d4554');
  },
};

const bench: ObjectArt = {
  w: 30,
  h: 16,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 30, 4, C.l!);
    rect(g, 0, 0, 30, 1, C.m!);
    rect(g, 0, 6, 30, 5, C.l!);
    rect(g, 0, 6, 30, 1, C.m!);
    rect(g, 2, 11, 2, 5, '#3d4554');
    rect(g, 26, 11, 2, 5, '#3d4554');
  },
};

const tree: ObjectArt = {
  w: 32,
  h: 42,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 13, 26, 6, 15, '#4a2c1a');
    rect(g, 14, 26, 2, 15, '#6b4022');
    const r = prng(17);
    const blobs: [number, number, number][] = [
      [16, 14, 13],
      [8, 18, 8],
      [24, 18, 8],
      [16, 6, 9],
      [10, 9, 7],
      [22, 9, 7],
    ];
    for (const [cx, cy, rad] of blobs)
      for (let y = -rad; y <= rad; y++)
        for (let x = -rad; x <= rad; x++) {
          if (x * x + y * y > rad * rad) continue;
          const shade = y > rad / 3 ? '#173326' : r() < 0.15 ? '#3f8a55' : '#2a5e3c';
          px(g, cx + x, cy + y, shade);
        }
    if (f === 1) {
      // hollow with a dead drop
      rect(g, 14, 31, 4, 5, '#140d0b');
      px(g, 16, 33, C.e!);
    }
  },
};

const bush: ObjectArt = {
  w: 16,
  h: 14,
  outline: true,
  draw(g) {
    const r = prng(23);
    for (let y = 0; y < 14; y++)
      for (let x = 0; x < 16; x++) {
        const dx = (x - 7.5) / 8;
        const dy = (y - 8) / 7;
        if (dx * dx + dy * dy > 1) continue;
        px(g, x, y, y > 9 ? '#173326' : r() < 0.2 ? '#3f8a55' : '#2a5e3c');
      }
  },
};

const crate: ObjectArt = {
  w: 16,
  h: 16,
  outline: true,
  draw(g) {
    rect(g, 0, 0, 16, 16, C.m!);
    rect(g, 0, 0, 16, 2, C.n!);
    rect(g, 0, 14, 16, 2, C.l!);
    for (let i = 0; i < 16; i++) px(g, i, i, C.l!);
    rect(g, 0, 0, 2, 16, C.l!);
    rect(g, 14, 0, 2, 16, C.l!);
  },
};

const barrel: ObjectArt = {
  w: 14,
  h: 16,
  outline: true,
  draw(g) {
    rect(g, 1, 0, 12, 16, '#2a5e8a');
    rect(g, 0, 2, 14, 12, '#2a5e8a');
    rect(g, 0, 3, 14, 1, '#1a3a5c');
    rect(g, 0, 12, 14, 1, '#1a3a5c');
    rect(g, 3, 0, 8, 2, '#3e7fb0');
    drawText(g, '!', 6, 6, C.e!);
  },
};

const vending: ObjectArt = {
  w: 16,
  h: 30,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 16, 30, '#b13e53');
    rect(g, 2, 3, 9, 18, '#1a1c2c');
    for (let y = 0; y < 4; y++)
      for (let x = 0; x < 3; x++) rect(g, 3 + x * 3, 4 + y * 4, 2, 3, [C.a!, C.e!, C.d!, C.i!][(x + y) % 4]!);
    rect(g, 12, 4, 3, 6, f ? C.a! : '#2b6b78');
    rect(g, 2, 23, 9, 4, '#0e0b16');
    rect(g, 0, 0, 16, 2, '#d0566f');
  },
};

const kiosk: ObjectArt = {
  w: 16,
  h: 30,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 2, 12, 12, 18, '#3d4554');
    rect(g, 2, 12, 12, 2, '#5f6b7d');
    rect(g, 0, 0, 16, 13, '#262a36');
    rect(g, 1, 1, 14, 10, f ? '#0f3b44' : '#0b2a31');
    rect(g, 2, 2, 8, 1, C.a!);
    rect(g, 2, 4, 11, 1, '#3aa8b8');
    rect(g, 2, 6, 6, 1, '#3aa8b8');
    if (f) rect(g, 2, 8, 2, 1, C.a!);
    rect(g, 5, 16, 6, 3, '#1a1c2c');
    px(g, 7, 17, C.d!);
  },
};

const antenna: ObjectArt = {
  w: 16,
  h: 48,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 7, 4, 2, 44, '#5f6b7d');
    for (let y = 10; y < 46; y += 6) {
      rect(g, 4, y, 8, 1, '#5f6b7d');
      px(g, 5, y + 1, '#3d4554');
      px(g, 10, y + 1, '#3d4554');
    }
    rect(g, 6, 0, 4, 4, f ? '#ff5566' : '#7a2e3a');
  },
};

const dish: ObjectArt = {
  w: 30,
  h: 30,
  outline: true,
  draw(g) {
    rect(g, 12, 18, 6, 12, '#5f6b7d');
    rect(g, 8, 27, 14, 3, '#3d4554');
    for (let y = 0; y < 20; y++)
      for (let x = 0; x < 30; x++) {
        const dx = (x - 15) / 15;
        const dy = (y - 9) / 9;
        if (dx * dx + dy * dy <= 1) px(g, x, y, dy > 0.3 ? '#94b0c2' : '#c7d3dc');
      }
    rect(g, 14, 7, 2, 6, '#3d4554');
    px(g, 15, 6, C.i!);
  },
};

const boat: ObjectArt = {
  w: 46,
  h: 22,
  frames: 2,
  outline: true,
  draw(g, f) {
    const dy = f;
    rect(g, 2, 8 + dy, 42, 10, '#e6e2c3');
    rect(g, 0, 8 + dy, 46, 3, '#b13e53');
    rect(g, 4, 18 + dy, 38, 3, '#94b0c2');
    rect(g, 14, 1 + dy, 16, 8, '#c7ccd6');
    rect(g, 16, 3 + dy, 4, 3, '#3a5a8a');
    rect(g, 22, 3 + dy, 4, 3, '#3a5a8a');
    drawText(g, 'OTP', 32, 12 + dy, '#29366f', 1);
  },
};

const barrierH: ObjectArt = {
  w: 16,
  h: 16,
  frames: 2,
  draw(g, f) {
    rect(g, 0, 5, 16, 6, f ? '#ff3e9a55' : '#ff3e9a33');
    for (let x = (f * 3) % 6; x < 16; x += 6) rect(g, x, 6, 2, 4, '#ff7ab8');
    rect(g, 0, 5, 16, 1, '#ff9acb');
    rect(g, 0, 10, 16, 1, '#ff9acb');
  },
};

const barrierV: ObjectArt = {
  w: 16,
  h: 16,
  frames: 2,
  draw(g, f) {
    rect(g, 5, 0, 6, 16, f ? '#ff3e9a55' : '#ff3e9a33');
    for (let y = (f * 3) % 6; y < 16; y += 6) rect(g, 6, y, 4, 2, '#ff7ab8');
    rect(g, 5, 0, 1, 16, '#ff9acb');
    rect(g, 10, 0, 1, 16, '#ff9acb');
  },
};

const pylon: ObjectArt = {
  w: 10,
  h: 20,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 2, 4, 6, 16, '#3d4554');
    rect(g, 1, 17, 8, 3, '#262a36');
    rect(g, 3, 0, 4, 5, f ? '#ff3e9a' : '#b13e78');
    rect(g, 4, 6, 2, 8, '#ff3e9a88');
  },
};

const drone: ObjectArt = {
  w: 18,
  h: 14,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 5, 4, 8, 6, '#3d4554');
    rect(g, 6, 5, 6, 3, '#262a36');
    px(g, 8, 6, '#ff5566');
    px(g, 9, 6, '#ff5566');
    rect(g, 0, 2, 5, 1, '#5f6b7d');
    rect(g, 13, 2, 5, 1, '#5f6b7d');
    const blade = f ? '#c7d3dc' : '#7a8699';
    rect(g, f ? 0 : 1, 0, 4, 1, blade);
    rect(g, f ? 14 : 13, 0, 4, 1, blade);
    rect(g, 7, 11, 4, 2, '#ff556655');
  },
};

const vault: ObjectArt = {
  w: 48,
  h: 52,
  frames: 3,
  outline: true,
  draw(g, f) {
    // plinth
    rect(g, 2, 40, 44, 12, '#262a36');
    rect(g, 2, 40, 44, 2, '#3d4554');
    rect(g, 0, 48, 48, 4, '#1a1c2c');
    // monolith
    rect(g, 8, 2, 32, 40, '#1d2236');
    rect(g, 8, 2, 32, 2, '#3a4466');
    rect(g, 10, 6, 28, 32, '#151a2b');
    // the five locks (shares)
    for (let i = 0; i < 5; i++) {
      const lit = f === 2 || (f === 1 && i < 3);
      rect(g, 13 + i * 5, 32, 3, 3, lit ? C.a! : '#2b3550');
    }
    if (f === 2) {
      rect(g, 16, 8, 16, 22, '#0b0d16');
      rect(g, 17, 9, 14, 20, '#102a33');
      rect(g, 22, 14, 4, 8, C.a!);
      rect(g, 20, 16, 8, 4, C.a!);
    } else {
      // engraved key glyph
      rect(g, 20, 12, 8, 8, '#2b3550');
      rect(g, 22, 14, 4, 4, '#151a2b');
      rect(g, 23, 20, 2, 9, '#2b3550');
      rect(g, 25, 24, 3, 2, '#2b3550');
      rect(g, 25, 27, 2, 2, '#2b3550');
    }
    drawText(g, 'LEGACY', 13, 42, f === 2 ? C.a! : '#5f6b7d', 1);
  },
};

const brickDrop: ObjectArt = {
  w: 32,
  h: 18,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 32, 18, '#6e2a1f');
    for (let y = 0; y < 18; y += 4)
      for (let x = (y / 4) % 2 ? 0 : 4; x < 32; x += 8) rect(g, x, y, 7, 3, '#9c4a32');
    rect(g, 0, 0, 32, 2, '#5f6b7d');
    if (f === 0) {
      // chalk mark on the loose brick
      rect(g, 12, 8, 7, 3, '#a8563a');
      px(g, 13, 9, '#e6e2c3');
      px(g, 14, 9, '#e6e2c3');
      px(g, 16, 9, '#e6e2c3');
    } else rect(g, 12, 8, 7, 3, '#140d0b');
  },
};

const mailbox: ObjectArt = {
  w: 12,
  h: 20,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 5, 9, 2, 11, '#3d4554');
    rect(g, 0, 0, 12, 10, '#29366f');
    rect(g, 0, 0, 12, 2, '#3b5dc9');
    rect(g, 2, 4, 8, 2, '#0e0b16');
    if (f === 0) px(g, 10, 6, C.e!);
  },
};

const serverRack: ObjectArt = {
  w: 16,
  h: 30,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 0, 0, 16, 30, '#1a1c2c');
    rect(g, 1, 1, 14, 28, '#262a36');
    for (let i = 0; i < 6; i++) {
      rect(g, 2, 2 + i * 4, 12, 3, '#1f2330');
      px(g, 3, 3 + i * 4, (i + f) % 3 ? C.d! : C.a!);
      px(g, 5, 3 + i * 4, (i + f) % 2 ? '#ff5566' : '#2b3550');
    }
  },
};

const trash: ObjectArt = {
  w: 12,
  h: 14,
  outline: true,
  draw(g) {
    rect(g, 1, 3, 10, 11, '#3d4554');
    rect(g, 0, 1, 12, 3, '#5f6b7d');
    for (let x = 3; x < 10; x += 3) rect(g, x, 5, 1, 7, '#262a36');
  },
};

const hydrant: ObjectArt = {
  w: 10,
  h: 14,
  outline: true,
  draw(g) {
    rect(g, 2, 3, 6, 10, '#b13e53');
    rect(g, 1, 12, 8, 2, '#7a2638');
    rect(g, 3, 0, 4, 3, '#b13e53');
    rect(g, 0, 6, 10, 2, '#7a2638');
  },
};

const telescope: ObjectArt = {
  w: 24,
  h: 26,
  outline: true,
  draw(g) {
    rect(g, 10, 14, 4, 10, '#3d4554');
    rect(g, 6, 23, 12, 3, '#262a36');
    for (let i = 0; i < 16; i++) rect(g, 2 + i, 12 - Math.floor(i * 0.6), 5, 4, i < 4 ? '#c7d3dc' : '#94b0c2');
    rect(g, 18, 0, 5, 5, '#c7d3dc');
  },
};

const masks: ObjectArt = {
  w: 20,
  h: 14,
  outline: true,
  draw(g) {
    const face = (x: number, col: string, smile: boolean) => {
      rect(g, x, 0, 9, 11, col);
      rect(g, x + 1, 11, 7, 2, col);
      rect(g, x + 2, 3, 2, 2, '#0e0b16');
      rect(g, x + 5, 3, 2, 2, '#0e0b16');
      if (smile) {
        rect(g, x + 2, 8, 5, 1, '#0e0b16');
        px(g, x + 1, 7, '#0e0b16');
        px(g, x + 7, 7, '#0e0b16');
      } else {
        rect(g, x + 2, 8, 5, 1, '#0e0b16');
        px(g, x + 1, 9, '#0e0b16');
        px(g, x + 7, 9, '#0e0b16');
      }
    };
    face(0, '#f4f4f4', true);
    face(11, '#c792ff', false);
  },
};

const coin: ObjectArt = {
  w: 14,
  h: 14,
  frames: 2,
  outline: true,
  draw(g, f) {
    const r = 6.5;
    for (let y = 0; y < 14; y++)
      for (let x = 0; x < 14; x++) {
        const dx = (x - 6.5) / (f ? r * 0.6 : r);
        const dy = (y - 6.5) / r;
        if (dx * dx + dy * dy <= 1) px(g, x, y, dy < -0.3 ? '#ffe7a8' : C.e!);
      }
    if (!f) drawText(g, 'B', 6, 4, '#b8862e');
  },
};

const projector: ObjectArt = {
  w: 16,
  h: 8,
  frames: 2,
  outline: true,
  draw(g, f) {
    rect(g, 1, 3, 14, 5, '#262a36');
    rect(g, 4, 1, 8, 3, '#3d4554');
    rect(g, 6, 0, 4, 2, f ? C.a! : '#2b8a96');
  },
};

const stall: ObjectArt = {
  w: 32,
  h: 30,
  outline: true,
  draw(g) {
    for (let x = 0; x < 32; x += 4) rect(g, x, 0, 4, 8, (x / 4) % 2 ? '#e6e2c3' : '#b13e53');
    rect(g, 0, 8, 32, 2, '#7a2638');
    rect(g, 2, 10, 2, 12, '#4a2c1a');
    rect(g, 28, 10, 2, 12, '#4a2c1a');
    rect(g, 0, 20, 32, 10, C.l!);
    rect(g, 0, 20, 32, 2, C.m!);
    for (let i = 0; i < 5; i++) rect(g, 3 + i * 6, 17, 4, 3, ['#ef7d57', '#a7f070', '#ffcd75', '#73eff7', '#ff3e9a'][i]!);
  },
};

const cone: ObjectArt = {
  w: 10,
  h: 12,
  outline: true,
  draw(g) {
    rect(g, 0, 10, 10, 2, '#262a36');
    rect(g, 4, 0, 2, 2, C.y!);
    rect(g, 3, 2, 4, 3, C.y!);
    rect(g, 2, 5, 6, 2, '#e6e2c3');
    rect(g, 2, 7, 6, 3, C.y!);
  },
};

const statue: ObjectArt = {
  w: 20,
  h: 34,
  outline: true,
  draw(g) {
    rect(g, 1, 26, 18, 8, '#3d4554');
    rect(g, 1, 26, 18, 2, '#5f6b7d');
    rect(g, 7, 2, 6, 6, '#94b0c2'); // head
    rect(g, 5, 8, 10, 12, '#94b0c2'); // body
    rect(g, 3, 10, 2, 8, '#7a8699');
    rect(g, 15, 6, 2, 10, '#7a8699'); // raised arm
    rect(g, 14, 2, 4, 4, '#c7d3dc'); // holding a key
    rect(g, 6, 20, 3, 6, '#7a8699');
    rect(g, 11, 20, 3, 6, '#7a8699');
  },
};

// ---------------------------------------------------------------------------
// Signs and buildings
// ---------------------------------------------------------------------------

export function neonSign(text: string, color: string, frames = 2): ObjectArt {
  const w = textWidth(text) + 8;
  return {
    w,
    h: 11,
    frames,
    draw(g, f) {
      rect(g, 0, 0, w, 11, '#0e0b16');
      rect(g, 1, 1, w - 2, 9, '#151a2b');
      drawText(g, text, 4, 3, f === 0 ? color : shade(color));
      rect(g, 0, 0, w, 1, '#262a36');
    },
  };
}

function shade(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.round(((n >> 16) & 255) * 0.75);
  const gg = Math.round(((n >> 8) & 255) * 0.75);
  const b = Math.round((n & 255) * 0.75);
  return `#${((r << 16) | (gg << 8) | b).toString(16).padStart(6, '0')}`;
}

export interface BuildingStyle {
  w: number;
  h: number;
  wall: string;
  wallDark: string;
  roof: string;
  trim: string;
  window: string;
  windowOff: string;
  door: string;
  seed: number;
  roofStyle?: 'flat' | 'dome' | 'peak' | 'sawtooth';
  doorAt?: number; // x of door
  pattern?: 'bricks' | 'panels' | 'plain' | 'wood';
}

export function building(st: BuildingStyle): ObjectArt {
  return {
    w: st.w,
    h: st.h,
    frames: 2,
    outline: true,
    draw(g, f) {
      const { w, h } = st;
      const roofH = st.roofStyle === 'dome' ? 22 : 10;
      const r = prng(st.seed);
      rect(g, 0, roofH, w, h - roofH, st.wall);
      if (st.pattern === 'bricks')
        for (let y = roofH; y < h; y += 4)
          for (let x = ((y - roofH) / 4) % 2 ? 0 : 4; x < w; x += 8) rect(g, x, y + 3, 7, 1, st.wallDark);
      if (st.pattern === 'panels') for (let x = 0; x < w; x += 12) rect(g, x, roofH, 1, h - roofH, st.wallDark);
      if (st.pattern === 'wood') for (let y = roofH; y < h; y += 3) rect(g, 0, y + 2, w, 1, st.wallDark);
      rect(g, 0, h - 3, w, 3, st.wallDark);
      // roof
      if (st.roofStyle === 'dome') {
        const cx = w / 2;
        const rad = Math.min(w / 2 - 2, 22);
        for (let y = 0; y < roofH + 2; y++)
          for (let x = 0; x < w; x++) {
            const dx = (x + 0.5 - cx) / rad;
            const dy = (y + 0.5 - (roofH + 2)) / (roofH + 2);
            if (dx * dx + dy * dy <= 1) px(g, x, y, dy < -0.6 ? '#c7d3dc' : st.roof);
          }
        rect(g, Math.floor(cx) - 1, 2, 3, roofH - 2, '#0e0b16');
        rect(g, 0, roofH, w, 3, st.trim);
      } else if (st.roofStyle === 'peak') {
        for (let y = 0; y < roofH; y++) {
          const inset = Math.max(0, roofH - 1 - y) * 2;
          rect(g, inset, y, w - inset * 2, 1, y % 3 === 2 ? st.trim : st.roof);
        }
      } else if (st.roofStyle === 'sawtooth') {
        for (let x = 0; x < w; x += 12)
          for (let y = 0; y < roofH; y++) rect(g, x, y, Math.min(12, Math.floor((y + 1) * 1.2)), 1, st.roof);
        rect(g, 0, roofH - 2, w, 2, st.trim);
      } else {
        rect(g, 0, 0, w, roofH, st.roof);
        rect(g, 0, roofH - 2, w, 2, st.trim);
        // rooftop clutter
        rect(g, 4 + Math.floor(r() * 10), 2, 8, 5, st.wallDark);
        rect(g, w - 18, 3, 6, 4, st.wallDark);
      }
      // windows
      const doorX = st.doorAt ?? Math.floor(w / 2) - 6;
      for (let y = roofH + 5; y < h - 18; y += 11)
        for (let x = 5; x < w - 9; x += 12) {
          if (x + 8 > doorX - 2 && x < doorX + 14 && y > h - 32) continue;
          const lit = r() < 0.6;
          const flicker = r() < 0.12;
          const on = lit && !(f === 1 && flicker);
          rect(g, x, y, 8, 7, '#0e0b16');
          rect(g, x + 1, y + 1, 6, 5, on ? st.window : st.windowOff);
          if (on) rect(g, x + 1, y + 1, 2, 1, '#ffffff66');
        }
      // door
      rect(g, doorX, h - 16, 12, 13, '#0e0b16');
      rect(g, doorX + 1, h - 15, 10, 12, st.door);
      rect(g, doorX + 1, h - 15, 10, 2, st.trim);
      px(g, doorX + 8, h - 9, '#ffffffaa');
      // awning
      rect(g, doorX - 3, h - 19, 18, 3, st.trim);
    },
  };
}

// ---------------------------------------------------------------------------

export const OBJECTS: Record<string, ObjectArt> = {
  bed,
  bookshelf,
  deskcrt: deskCrt,
  dresser,
  fireplace,
  plant,
  armchair,
  sidetable: sideTable,
  photo,
  rug,
  coatrack: coatRack,
  boxes,
  note,
  lamp,
  bench,
  tree,
  bush,
  crate,
  barrel,
  vending,
  kiosk,
  antenna,
  dish,
  boat,
  barrier_h: barrierH,
  barrier_v: barrierV,
  pylon,
  drone,
  vault,
  brickdrop: brickDrop,
  mailbox,
  server: serverRack,
  trash,
  hydrant,
  telescope,
  masks,
  coin,
  projector,
  stall,
  cone,
  statue,
};

export function drawObjectSheet(art: ObjectArt): HTMLCanvasElement {
  const frames = art.frames ?? 1;
  const pad = art.outline ? 1 : 0;
  const fw = art.w + 2 * pad;
  const fh = art.h + 2 * pad;
  const sheet = makeCanvas(fw * frames, fh);
  const ctx = ctx2d(sheet);
  for (let f = 0; f < frames; f++) {
    const tmp = makeCanvas(fw, fh);
    const t = ctx2d(tmp);
    t.translate(pad, pad);
    art.draw(t, f);
    t.setTransform(1, 0, 0, 1, 0, 0);
    if (art.outline) addOutline(tmp);
    ctx.drawImage(tmp, f * fw, 0);
  }
  return sheet;
}

export function objectFrameSize(art: ObjectArt): { w: number; h: number } {
  const pad = art.outline ? 1 : 0;
  return { w: art.w + 2 * pad, h: art.h + 2 * pad };
}

export { paint, type Rows };
