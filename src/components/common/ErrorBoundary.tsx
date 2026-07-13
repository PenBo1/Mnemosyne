import { Component, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/locales/i18n";

interface ErrorBoundaryProps {
  children: ReactNode;
  // 可选：自定义 fallback 渲染函数（接收错误和重置函数）
  fallback?: (error: Error, reset: () => void) => ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

// ErrorBoundary：捕获子树渲染错误，避免整应用白屏崩溃
// 用 key 重置：父组件改变传入的 key 时，组件会重新挂载并清空错误状态
class ErrorBoundaryImpl extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error("[ErrorBoundary] render error", error, info);
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
