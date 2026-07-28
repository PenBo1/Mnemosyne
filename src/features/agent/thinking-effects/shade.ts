const WIDTH = 9;

const TEMPORAL_SPEED = {
  CRAWL: 0.05,
  SLOW: 0.12,
  GENTLE: 0.25,
  MODERATE: 0.4,
  FAST: 0.7,
  INTENSE: 1.3,
} as const;

const SPATIAL_FREQUENCY = {
  LOW: 0.6,
  HIGH: 1.2,
  DENSE: 2.1,
  EXTRA_DENSE: 6.0,
} as const;

const SHADES = ['░', '▒', '▓', '█'] as const;

class ShadeFrame {
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

  add(x: number, density: number): void {
    if (x < this.width) {
      this.cells[x] = Math.max(0.0, Math.min(1.0, this.cells[x] + density));
    }
  }

  clear(): void {
    this.cells.fill(0.0);
  }

  render(): string {
    const row = this.cells
      .map((density) => {
        const idx = Math.round(density * (SHADES.length - 1));
        return SHADES[Math.min(idx, SHADES.length - 1)];
      })
      .join('');
    return row;
  }
}

interface EffectState {
  frame: number;
  cycle: number;
}

function createEffectState(_seed: number, cycle: number): EffectState {
  return {
    frame: 0,
    cycle,
  };
}

function advanceState(state: EffectState): void {
  state.frame = (state.frame + 1) % state.cycle;
}

export class ShadeFire {
  private state: EffectState;
  private frame: ShadeFrame;

  constructor() {
    this.state = createEffectState(42, 20);
    this.frame = new ShadeFrame(WIDTH);
  }

  name(): string {
    return 'shade-fire';
  }

  description(): string {
    return 'Fire effect using shade characters';
  }

  step(): string {
    const result = this.render();
    advanceState(this.state);
    return result;
  }

  private render(): string {
    this.frame.clear();
    for (let x = 0; x < WIDTH; x++) {
      const v =
        Math.sin(this.state.frame * TEMPORAL_SPEED.FAST + x * SPATIAL_FREQUENCY.DENSE) * 0.3 +
        Math.sin(this.state.frame * TEMPORAL_SPEED.INTENSE + x * SPATIAL_FREQUENCY.HIGH) * 0.2 +
        0.5;
      this.frame.set(x, Math.max(0.0, Math.min(1.0, v)));
    }
    return this.frame.render();
  }
}

export class ShadeWave {
  private state: EffectState;
  private frame: ShadeFrame;

  constructor() {
    this.state = createEffectState(42, 10);
    this.frame = new ShadeFrame(WIDTH);
  }

  name(): string {
    return 'shade-wave';
  }

  description(): string {
    return 'Sine wave using shade characters';
  }

  step(): string {
    const result = this.render();
    advanceState(this.state);
    return result;
  }

  private render(): string {
    this.frame.clear();
    for (let i = 0; i < WIDTH; i++) {
      const v = Math.sin((i + this.state.frame) * SPATIAL_FREQUENCY.LOW) / 2.0 + 0.5;
      this.frame.set(i, v);
    }
    return this.frame.render();
  }
}

export class ShadeBreathe {
  private state: EffectState;
  private frame: ShadeFrame;

  constructor() {
    this.state = createEffectState(42, 18);
    this.frame = new ShadeFrame(WIDTH);
  }

  name(): string {
    return 'shade-breathe';
  }

  description(): string {
    return 'Breathing shade animation';
  }

  step(): string {
    const result = this.render();
    advanceState(this.state);
    return result;
  }

  private render(): string {
    const phase = this.state.frame % 18;
    const v = Math.sin((phase * Math.PI) / 10.0) / 2.0 + 0.5;
    this.frame.clear();
    for (let i = 0; i < WIDTH; i++) {
      this.frame.set(i, v);
    }
    return this.frame.render();
  }
}