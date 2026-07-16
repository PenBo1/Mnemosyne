import { Component, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/locales/i18n";

interface ErrorBoundaryProps {
  children: ReactNode;
  // 可选：自定义 fallback 渲染函数（接收错误和重置函数）
  fallback?: (error: Error, reset: () => void) => ReactNode;
  // 可选：当这些值变化时自动重置错误状态（避免用 key 强制重挂载整棵子树）
  resetKeys?: unknown[];
}

interface ErrorBoundaryState {
  error: Error | null;
}

// ErrorBoundary：捕获子树渲染错误，避免整应用白屏崩溃
// 支持 resetKeys 自动重置：当 resetKeys 值变化时清空错误状态，无需 key 强制重挂载
class ErrorBoundaryImpl extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error("[ErrorBoundary] render error", error, info);
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps) {
    if (this.state.error && this.props.resetKeys) {
      const changed = this.props.resetKeys.some(
        (key, idx) => key !== prevProps.resetKeys?.[idx],
      );
      if (changed) {
        this.setState({ error: null });
      }
    }
  }

  reset = () => {
    this.setState({ error: null });
  };

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    if (this.props.fallback) return this.props.fallback(error, this.reset);
    return <DefaultFallback error={error} reset={this.reset} />;
  }
}

// 默认 fallback UI（独立组件以使用 useI18n hook，class 内不能用 hook）
function DefaultFallback({ error, reset }: { error: Error; reset: () => void }) {
  const { t } = useI18n();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <div className="text-sm text-[var(--text-tertiary)]">{t.common.pageError}</div>
      <div className="max-w-md text-xs text-muted-foreground">
        {error.message}
      </div>
      <Button variant="outline" size="sm" onClick={reset}>
        {t.common.retry}
      </Button>
    </div>
  );
}

export const ErrorBoundary = ErrorBoundaryImpl;
