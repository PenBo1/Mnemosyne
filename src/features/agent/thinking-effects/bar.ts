const BARS = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'] as const;
const WIDTH = 9;

const TEMPORAL_SPEED = {
  GENTLE: 0.25,
  FAST: 0.7,
} as const;

const SPATIAL_FREQUENCY = {
  LOW: 0.6,
} as const;

const CYCLE_LENGTH = 25;

class BarFrame {
  private cells: number[];

  constructor(private width: number) {
    this.cells = new Array(width).fill(0);
  }

  set(x: number, density: number): void {
    if (x < this.width) {
      this.cells[x] = Math.max(0, Math.min(1, density));
    }
  }

  render(): string {
    return this.cells
      .map((density) => {
        const idx = Math.round(density * (BARS.length - 1));
        const clampedIdx = Math.min(idx, BARS.length - 1);
        return BARS[clampedIdx];
      })
      .join('');
  }
}

function seededRandom(seed: number): () => number {
  const m = 0x80000000;
  const a = 1103515245;
  const c = 12345;
  let state = seed;

  return () => {
    state = (a * state + c) % m;
    return state / m;
  };
}

function randomRange(
  random: () => number,
  min: number,
  max: number
): number {
  return min + random() * (max - min);
}

export class BarBounce {
  private frame = 0;
  private phases: number[];
  private speeds: number[];
  private random: () => number;

  constructor(seed = 42) {
    this.random = seededRandom(seed);
    this.phases = Array.from({ length: WIDTH }, () =>
      randomRange(this.random, 0, Math.PI * 2)
    );
    this.speeds = Array.from({ length: WIDTH }, () =>
      randomRange(this.random, TEMPORAL_SPEED.GENTLE, TEMPORAL_SPEED.FAST)
    );
  }

  step(): string {
    const frame = new BarFrame(WIDTH);

    for (let i = 0; i < WIDTH; i++) {
      const value =
        (Math.sin(this.phases[i] + this.frame * this.speeds[i]) / 2) + 0.5;
      frame.set(i, value);
    }

    this.frame = (this.frame + 1) % CYCLE_LENGTH;
    return frame.render();
  }

  reset(): void {
    this.frame = 0;
  }
}

export class BarWave {
  private frame = 0;
  private spatialOffsets: number[];

  constructor() {
    this.spatialOffsets = Array.from(
      { length: WIDTH },
      (_, i) => i * SPATIAL_FREQUENCY.LOW
    );
  }

  step(): string {
    const frame = new BarFrame(WIDTH);

    for (let i = 0; i < WIDTH; i++) {
      const value =
        (Math.sin(this.spatialOffsets[i] + this.frame * TEMPORAL_SPEED.GENTLE) /
          2) +
        0.5;
      frame.set(i, value);
    }

    this.frame = (this.frame + 1) % CYCLE_LENGTH;
    return frame.render();
  }

  reset(): void {
    this.frame = 0;
  }
}

export class BarSeeSaw {
  private frame = 0;
  private ratios: number[];

  constructor() {
    this.ratios = Array.from({ length: WIDTH }, (_, i) =>
      WIDTH > 1 ? i / (WIDTH - 1) : 0.5
    );
  }

  step(): string {
    const frame = new BarFrame(WIDTH);
    const t = (Math.sin(this.frame * TEMPORAL_SPEED.GENTLE) / 2) + 0.5;

    for (let i = 0; i < WIDTH; i++) {
      const ratio = this.ratios[i];
      const value = t * (1 - ratio) + (1 - t) * ratio;
      frame.set(i, value);
    }

    this.frame = (this.frame + 1) % CYCLE_LENGTH;
    return frame.render();
  }

  reset(): void {
    this.frame = 0;
  }
}

export type BarEffect = BarBounce | BarWave | BarSeeSaw;

export const BarEffectNames = {
  BOUNCE: 'bar-bounce',
  WAVE: 'bar-wave',
  SEESAW: 'bar-seesaw',
} as const;

export function createBarEffect(
  type: (typeof BarEffectNames)[keyof typeof BarEffectNames],
  seed?: number
): BarEffect {
  switch (type) {
    case BarEffectNames.BOUNCE:
      return new BarBounce(seed);
    case BarEffectNames.WAVE:
      return new BarWave();
    case BarEffectNames.SEESAW:
      return new BarSeeSaw();
    default:
      throw new Error(`Unknown bar effect type: ${type}`);
  }
}