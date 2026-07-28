/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 生命周期管理器 - 应用生命周期事件处理组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useLifecycle } from "./lifecycle";

// ── 类型定义 ────────────────────────────────────────────────────────────────

export interface LifecycleManagerProps {
  onWindowClose?: () => void;
  onConfigChanged?: (config: unknown) => void;
  onError?: (error: Error) => void;
}

// ── 组件实现 ────────────────────────────────────────────────────────────────

export function LifecycleManager({ 
  onWindowClose, 
  onConfigChanged, 
  onError 
}: LifecycleManagerProps): null {
  useLifecycle({
    onWindowClose,
    onConfigChanged,
    onError,
  });
  
  return null;
}