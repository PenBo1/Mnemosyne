const DOTS: string[] = ['·', '∘', '•', '○', '●'];

const WIDTH = 9;

const temporal_speed = {
  CRAWL: 0.05,
  SLOW: 0.12,
  GENTLE: 0.25,
  MODERATE: 0.4,
  FAST: 0.7,
  INTENSE: 1.3,
};

const spatial_frequency = {
  LOW: 0.6,
  HIGH: 1.2,
  DENSE: 2.1,
  EXTRA_DENSE: 6.0,
};

class DotFrame {
  private width: number;
  private cells: number[];

  constructor(width: number) {
    this.width = width;
    this.cells = new Array(width).fill(0.0);
  }

  set(x: number, density: number): void {
    if (x < this.width) {
      this.cells[x] = Math.max(0.0, Math.min(1.0, density));
    }
  }

  render(): string {
    let row = '';
    for (let c = 0; c < this.width; c++) {
      const idx = Math.min(
        DOTS.length - 1,
        Math.round(this.cells[c] * (DOTS.length - 1))
      );
      row += DOTS[idx];
    }
    return row;
  }
}

class EffectState {
  frame: number = 0;
  private cycle: number;
  private seed: number;

  constructor(seed: number, cycle: number) {
    this.seed = seed;
    this.cycle = cycle;
  }

  advance(): void {
    this.frame = (this.frame + 1) % this.cycle;
  }

  randomRange(min: number, max: number): number {
    const x = Math.sin(this.seed++) * 10000;
    return min + (x - Math.floor(x)) * (max - min);
  }
}

export interface DotEffect {
  name(): string;
  description(): string;
  step(): string;
}

export class DotWave implements DotEffect {
  private state: EffectState;

  constructor() {
    this.state = new EffectState(42, 10);
  }

  name(): string {
    return 'dot-wave';
  }

  description(): string {
    return 'Sine wave using dot characters';
  }

  step(): string {
    const f = new DotFrame(WIDTH);
    for (let i = 0; i < WIDTH; i++) {
      const v = Math.sin((i + this.state.frame) * spatial_frequency.LOW) / 2 + 0.5;
      f.set(i, v);
    }
    this.state.advance();
    return f.render();
  }
}

export class DotPulse implements DotEffect {
  private state: EffectState;
  private static readonly CX = WIDTH / 2;

  constructor() {
    this.state = new EffectState(42, 2 * WIDTH);
  }

  name(): string {
    return 'dot-pulse';
  }

  description(): string {
    return 'Expanding ring in dot characters';
  }

  step(): string {
    const f = new DotFrame(WIDTH);
    const cycleLength = 2 * WIDTH;
    const frameMod = this.state.frame % cycleLength;
    const ring = frameMod < cycleLength / 2
      ? frameMod
      : cycleLength - frameMod - 1;

    for (let i = 0; i < WIDTH; i++) {
      const dist = Math.abs(i - DotPulse.CX + 0.5);
      const v = Math.max(0.0, Math.min(1.0, (ring - dist + 1.0) / 3.0));
      f.set(i, v);
    }

    this.state.advance();
    return f.render();
  }
}

export class DotBounce implements DotEffect {
  private state: EffectState;
  private phases: number[];
  private speeds: number[];

  constructor() {
    this.state = new EffectState(42, 25);
    this.phases = [];
    this.speeds = [];

    for (let i = 0; i < WIDTH; i++) {
      this.phases.push(this.state.randomRange(0, Math.PI * 2));
      this.speeds.push(this.state.randomRange(temporal_speed.GENTLE, temporal_speed.FAST));
    }
  }

  name(): string {
    return 'dot-bounce';
  }

  description(): string {
    return 'Bouncing dots with random phases';
  }

  step(): string {
    const f = new DotFrame(WIDTH);
    for (let i = 0; i < WIDTH; i++) {
      const v = Math.sin(this.phases[i] + this.state.frame * this.speeds[i]) / 2 + 0.5;
      f.set(i, v);
    }
    this.state.advance();
    return f.render();
  }
}

export const DOT_EFFECTS = {
  DotWave,
  DotPulse,
  DotBounce,
};