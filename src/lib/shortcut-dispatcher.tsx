import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  useCallback,
  useMemo,
  type ReactNode,
} from "react";
import { getShortcutsOverrides } from "@/services/settings";
import { SHORTCUTS, matchBinding, type ShortcutId, type KeyBinding } from "@/lib/shortcuts";

type Handler = () => void;

interface ShortcutContextValue {
  /** 当前生效绑定（defaults + overrides），供组件局部匹配 */
  bindings: Record<ShortcutId, KeyBinding[]>;
  /** 注册全局快捷键 handler，返回注销函数 */
  register: (id: ShortcutId, handler: Handler) => () => void;
}

const ShortcutContext = createContext<ShortcutContextValue | null>(null);

/** 焦点是否在可编辑元素内 */
function isEditable(el: Element | null): boolean {
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select" || (el as HTMLElement).isContentEditable;
}

/**
 * 快捷键派发中心：加载用户覆盖配置，注册全局 keydown 监听，
 * 匹配后调用对应 handler。输入框内只响应带修饰键的快捷键，
 * 纯键（如 Enter/ArrowUp）交给组件自己处理。
 */
export function ShortcutProvider({ children }: { children: ReactNode }) {
  const [overrides, setOverrides] = useState<Record<string, KeyBinding[]>>({});
  const handlersRef = useRef(new Map<ShortcutId, Handler>());

  // 加载用户覆盖配置
  useEffect(() => {
    let cancelled = false;
    void getShortcutsOverrides().then((map) => {
      if (!cancelled) setOverrides(map);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // 合并默认绑定与覆盖
  const bindings = useMemo(() => {
    const map = {} as Record<ShortcutId, KeyBinding[]>;
    for (const s of SHORTCUTS) {
      map[s.id] = overrides[s.id] ?? s.defaultBindings;
    }
    return map;
  }, [overrides]);

  // 全局 keydown 派发
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const inEditable = isEditable(e.target as Element);
      for (const s of SHORTCUTS) {
        const list = bindings[s.id];
        for (const b of list) {
          if (!matchBinding(e, b)) continue;
          // 可编辑元素内：纯键（无 Ctrl/Meta/Alt）交给组件处理
          if (inEditable && !b.ctrl && !b.meta && !b.alt) continue;
          const handler = handlersRef.current.get(s.id);
          if (handler) {
            e.preventDefault();
            handler();
            return;
          }
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [bindings]);

  const register = useCallback((id: ShortcutId, handler: Handler) => {
    handlersRef.current.set(id, handler);
    return () => {
      if (handlersRef.current.get(id) === handler) {
        handlersRef.current.delete(id);
      }
    };
  }, []);

  const value = useMemo(() => ({ bindings, register }), [bindings, register]);

  return <ShortcutContext.Provider value={value}>{children}</ShortcutContext.Provider>;
}

/** 读取当前生效绑定（组件局部按键匹配用） */
export function useShortcutBindings(): Record<ShortcutId, KeyBinding[]> {
  const ctx = useContext(ShortcutContext);
  return ctx?.bindings ?? ({} as Record<ShortcutId, KeyBinding[]>);
}

/** 注册全局快捷键 handler */
export function useShortcut(id: ShortcutId, handler: Handler): void {
  const ctx = useContext(ShortcutContext);
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!ctx) return;
    return ctx.register(id, () => handlerRef.current());
  }, [ctx, id]);
}
