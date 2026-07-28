export interface Effect {
  name: string;
  description: string;
  cycleLength: number;
  step(): string;
}

export type EffectClass<T extends Effect> = new () => T;

export type EffectFamily = 'braille' | 'shade' | 'bar' | 'vblock' | 'dot';

export interface EffectConfig {
  width: number;
  height: number;
  fps: number;
}

export const DEFAULT_CONFIG: EffectConfig = {
  width: 9,
  height: 1,
  fps: 10,
};