import type { Effect } from './types';

const VBLOCKS = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'] as const;

const WIDTH = 9;

const spatialFrequency = {
  LOW: 0.6,
};

class VBlockFrame {
  private width: number;
  private cells: number[];

  constructor(width: number) {
    this.width = width;
    this.cells = new Array(width).fill(0);
  }

  set(x: number, density: number): void {
    if (x < this.width) {
      this.cells[x] = Math.max(0, Math.min(1, density));
    }
  }

  clear(): void {
    this.cells.fill(0);
  }

  render(): string {
    const row = this.cells
      .map((density) => {
        const idx = Math.round(density * (VBLOCKS.length - 1));
        const clampedIdx = Math.min(idx, VBLOCKS.length - 1);
        return VBLOCKS[clampedIdx];
      })
      .join('');
    return row;
  }
}

class EffectState {
  frame: number;
  private cycle: number;

  constructor(cycle: number) {
    this.frame = 0;
    this.cycle = cycle;
  }

  advance(): void {
    this.frame = (this.frame + 1) % this.cycle;
  }
}

export class VBlockWave implements Effect {
  readonly name = 'vblock-wave';
  readonly description = 'Sine wave using vertical blocks';
  readonly cycleLength = 10;

  private state: EffectState;

  constructor() {
    this.state = new EffectState(this.cycleLength);
  }

  step(): string {
    const result = this.render();
    this.state.advance();
    return result;
  }

  private render(): string {
    const f = new VBlockFrame(WIDTH);
    for (let i = 0; i < WIDTH; i++) {
      const v = (Math.sin((i + this.state.frame) * spatialFrequency.LOW) / 2) + 0.5;
      f.set(i, v);
    }
    return f.render();
  }
}

export class VBlockScanner implements Effect {
  readonly name = 'vblock-scanner';
  readonly description = 'Scanning beam across using vertical blocks';
  readonly cycleLength: number;

  private static readonly FILL_CYCLE = WIDTH * 2;
  private state: EffectState;

  constructor() {
    this.cycleLength = VBlockScanner.FILL_CYCLE;
    this.state = new EffectState(this.cycleLength);
  }

  step(): string {
    const result = this.render();
    this.state.advance();
    return result;
  }

  private render(): string {
    const f = new VBlockFrame(WIDTH);
    const pos = this.state.frame % VBlockScanner.FILL_CYCLE;
    for (let i = 0; i < WIDTH; i++) {
      if (i < pos) {
        const dist = pos - i;
        const v = Math.max(0, 1 - (dist - 1) / (WIDTH - 1));
        f.set(i, v);
      }
    }
    return f.render();
  }
}

export class VBlockPulse implements Effect {
  readonly name = 'vblock-pulse';
  readonly description = 'Expanding pulse in vertical blocks';
  readonly cycleLength = WIDTH + 4;

  private static readonly CENTER = (WIDTH - 1) / 2;
  private state: EffectState;

  constructor() {
    this.state = new EffectState(this.cycleLength);
  }

  step(): string {
    const result = this.render();
    this.state.advance();
    return result;
  }

  private render(): string {
    const f = new VBlockFrame(WIDTH);
    const age = this.state.frame;
    const intensity = Math.min(1, this.state.frame / (this.cycleLength * 0.6));
    for (let i = 0; i < WIDTH; i++) {
      const dist = Math.abs(i - VBlockPulse.CENTER);
      const wave = Math.max(0, 1 - (dist - age + 2) * 0.25);
      f.set(i, wave * intensity);
    }
    return f.render();
  }
}