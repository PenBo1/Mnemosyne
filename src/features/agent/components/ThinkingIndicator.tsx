/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ThinkingIndicator - 思考状态指示器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { CSSProperties } from 'react';
import { useThinkingAnimation } from '../hooks/useThinkingAnimation';
import {
  BrailleSpin,
  VBlockWave,
  ShadeFireAdapter,
  BarBounceAdapter,
  DotPulseAdapter,
} from '../thinking-effects';
import type { Effect, EffectClass } from '../thinking-effects/types';

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface ThinkingIndicatorProps {
  effect?: string | EffectClass<Effect>;
  className?: string;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

const DEFAULT_EFFECTS: Record<string, EffectClass<Effect>> = {
  'braille-spin': BrailleSpin,
  'shade-fire': ShadeFireAdapter,
  'bar-bounce': BarBounceAdapter,
  'vblock-wave': VBlockWave,
  'dot-pulse': DotPulseAdapter,
};

/**
 * 指示器样式，避免每次渲染重建对象
 */
const INDICATOR_STYLE: CSSProperties = {
  fontFamily: 'Fira Code, Cascadia Code, monospace',
  display: 'inline-block',
  minWidth: '9ch',
  fontSize: '1.2em',
  letterSpacing: '0.05em',
};

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 思考状态指示器，显示动态的思考动画效果
 */
export function ThinkingIndicator({
  effect = 'shade-fire',
  className,
}: ThinkingIndicatorProps) {
  const EffectCls =
    typeof effect === 'string'
      ? DEFAULT_EFFECTS[effect] || ShadeFireAdapter
      : effect;

  const frame = useThinkingAnimation(EffectCls);

  return (
    <span
      className={className}
      style={INDICATOR_STYLE}
    >
      {frame}
    </span>
  );
}