import type { Effect } from './types';

const WIDTH = 9;
const HEIGHT = 1;

const DOT_MAP: number[][] = [
  [0x01, 0x08],
  [0x02, 0x10],
  [0x04, 0x20],
  [0x40, 0x80],
];

class BrailleFrame {
  private width: number;
  private height: number;
  private cells: number[];

  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.cells = new Array(width * height).fill(0);
  }

  set(x: number, y: number): void {
    const col = Math.floor(x / 2);
    const dotCol = x % 2;
    const row = Math.floor(y / 4);
    const dotRow = y % 4;
    const idx = row * this.width + col;
    if (idx < this.cells.length) {
      this.cells[idx] |= DOT_MAP[dotRow][dotCol];
    }
  }

  unset(x: number, y: number): void {
    const col = Math.floor(x / 2);
    const dotCol = x % 2;
    const row = Math.floor(y / 4);
    const dotRow = y % 4;
    const idx = row * this.width + col;
    if (idx < this.cells.length) {
      this.cells[idx] &= ~DOT_MAP[dotRow][dotCol];
    }
  }

  clear(): void {
    this.cells.fill(0);
  }

  render(): string[] {
    const rows: string[] = [];
    for (let r = 0; r < this.height; r++) {
      const start = r * this.width;
      const row: string[] = [];
      for (let c = 0; c < this.width; c++) {
        const val = this.cells[start + c] & 0xff;
        row.push(String.fromCodePoint(0x2800 + val));
      }
      rows.push(row.join(''));
    }
    return rows;
  }
}

const TRAIL_SHORT = 3;

export class BrailleSpin implements Effect {
  name = 'braille-spin';
  description = 'Braille spinner, same char repeated';
  cycleLength = 8;

  private frame = 0;

  private static readonly PATH: [number, number][] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [0, 3],
    [1, 3],
    [1, 2],
    [1, 1],
    [1, 0],
  ];

  step(): string {
    const result = this.render();
    this.frame = (this.frame + 1) % this.cycleLength;
    return result;
  }

  private render(): string {
    const t = TRAIL_SHORT;
    const f = new BrailleFrame(WIDTH, HEIGHT);
    for (let cx = 0; cx < WIDTH * 2; cx += 4) {
      for (let i = 0; i < t; i++) {
        const [dx, dy] = BrailleSpin.PATH[(this.frame + i) % BrailleSpin.PATH.length];
        f.set(cx + dx, dy);
      }
    }
    return f.render().join('\n');
  }
}

export class BrailleWave implements Effect {
  name = 'braille-wave';
  description = 'Sine wave across braille cells';
  cycleLength = 8;

  private frame = 0;

  step(): string {
    const result = this.render();
    this.frame = (this.frame + 1) % this.cycleLength;
    return result;
  }

  private render(): string {
    const f = new BrailleFrame(WIDTH, HEIGHT);
    for (let i = 0; i < WIDTH; i++) {
      const phase = (this.frame + i) % 8;
      const y = phase < 4 ? phase : 7 - phase;
      f.set(i * 2, y);
    }
    return f.render().join('\n');
  }
}

export class BrailleBreathe implements Effect {
  name = 'braille-breathe';
  description = 'Breathing braille expansion';
  cycleLength = 9;

  private frame = 0;

  private static readonly PIXEL_ORDER: [number, number][] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [1, 0],
    [1, 1],
    [1, 2],
    [0, 3],
    [1, 3],
  ];

  step(): string {
    const result = this.render();
    this.frame = (this.frame + 1) % this.cycleLength;
    return result;
  }

  private render(): string {
    const cl = this.cycleLength;
    const phase = this.frame % cl;
    const f = new BrailleFrame(WIDTH, HEIGHT);
    for (let i = 0; i < WIDTH; i++) {
      const dist = Math.abs(i - Math.floor(WIDTH / 2));
      const ci = (phase + dist) % cl;
      const count = ci < 5 ? ci : cl - ci;
      const n = Math.floor((count * 8) / 5);
      for (let b = 0; b < n; b++) {
        const [px, py] = BrailleBreathe.PIXEL_ORDER[b];
        f.set(i * 2 + px, py);
      }
    }
    return f.render().join('\n');
  }
}