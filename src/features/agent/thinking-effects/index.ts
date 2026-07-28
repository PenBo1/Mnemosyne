export type { Effect, EffectClass, EffectFamily, EffectConfig } from './types';
export { DEFAULT_CONFIG } from './types';

export { BrailleSpin, BrailleWave, BrailleBreathe } from './braille';

export { ShadeFire, ShadeWave, ShadeBreathe } from './shade';

export { BarBounce, BarWave, BarSeeSaw } from './bar';
export type { BarEffect } from './bar';
export { BarEffectNames, createBarEffect } from './bar';

export { VBlockWave, VBlockScanner, VBlockPulse } from './vblock';

export { DotWave, DotPulse, DotBounce } from './dot';
export type { DotEffect } from './dot';
export { DOT_EFFECTS } from './dot';

import type { Effect } from './types';
import { BrailleSpin, BrailleWave, BrailleBreathe } from './braille';
import { ShadeFire, ShadeWave, ShadeBreathe } from './shade';
import { BarBounce, BarWave, BarSeeSaw } from './bar';
import { VBlockWave, VBlockScanner, VBlockPulse } from './vblock';
import { DotWave, DotPulse, DotBounce } from './dot';

class ShadeFireAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: ShadeFire;

  constructor() {
    this.effect = new ShadeFire();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 20;
  }

  step(): string {
    return this.effect.step();
  }
}

class ShadeWaveAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: ShadeWave;

  constructor() {
    this.effect = new ShadeWave();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 10;
  }

  step(): string {
    return this.effect.step();
  }
}

class ShadeBreatheAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: ShadeBreathe;

  constructor() {
    this.effect = new ShadeBreathe();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 18;
  }

  step(): string {
    return this.effect.step();
  }
}

class BarBounceAdapter implements Effect {
  name = 'bar-bounce';
  description = 'Bouncing bars with random phases';
  cycleLength = 25;
  private effect: BarBounce;

  constructor() {
    this.effect = new BarBounce();
  }

  step(): string {
    return this.effect.step();
  }
}

class BarWaveAdapter implements Effect {
  name = 'bar-wave';
  description = 'Sine wave using bar characters';
  cycleLength = 25;
  private effect: BarWave;

  constructor() {
    this.effect = new BarWave();
  }

  step(): string {
    return this.effect.step();
  }
}

class BarSeeSawAdapter implements Effect {
  name = 'bar-seesaw';
  description = 'See-saw bar animation';
  cycleLength = 25;
  private effect: BarSeeSaw;

  constructor() {
    this.effect = new BarSeeSaw();
  }

  step(): string {
    return this.effect.step();
  }
}

class DotWaveAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: DotWave;

  constructor() {
    this.effect = new DotWave();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 10;
  }

  step(): string {
    return this.effect.step();
  }
}

class DotPulseAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: DotPulse;

  constructor() {
    this.effect = new DotPulse();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 18;
  }

  step(): string {
    return this.effect.step();
  }
}

class DotBounceAdapter implements Effect {
  name: string;
  description: string;
  cycleLength: number;
  private effect: DotBounce;

  constructor() {
    this.effect = new DotBounce();
    this.name = this.effect.name();
    this.description = this.effect.description();
    this.cycleLength = 25;
  }

  step(): string {
    return this.effect.step();
  }
}

export const EFFECTS: Array<new () => Effect> = [
  BrailleSpin,
  BrailleWave,
  BrailleBreathe,
  ShadeFireAdapter,
  ShadeWaveAdapter,
  ShadeBreatheAdapter,
  BarBounceAdapter,
  BarWaveAdapter,
  BarSeeSawAdapter,
  VBlockWave,
  VBlockScanner,
  VBlockPulse,
  DotWaveAdapter,
  DotPulseAdapter,
  DotBounceAdapter,
];

export function createRandomEffect(): Effect {
  const EffectClass = EFFECTS[Math.floor(Math.random() * EFFECTS.length)];
  return new EffectClass();
}

export { ShadeFireAdapter, BarBounceAdapter, DotPulseAdapter };