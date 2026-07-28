import { useEffect, useRef, useState } from 'react';
import type { Effect, EffectClass } from '../thinking-effects/types';

const FPS = 10;
const INTERVAL = 1000 / FPS;

export function useThinkingAnimation<T extends Effect>(EffectCls: EffectClass<T>): string {
  const [frame, setFrame] = useState('');
  const instanceRef = useRef<T | null>(null);

  useEffect(() => {
    const instance = new EffectCls();
    instanceRef.current = instance;

    let lastTime = 0;
    let rafId: number;

    const tick = (time: number) => {
      if (time - lastTime >= INTERVAL) {
        setFrame(instance.step());
        lastTime = time;
      }
      rafId = requestAnimationFrame(tick);
    };

    rafId = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(rafId);
    };
  }, [EffectCls]);

  return frame;
}