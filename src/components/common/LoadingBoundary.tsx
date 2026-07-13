import { Suspense } from "react";

function PageLoading() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-[var(--text-tertiary)]">
      Loading...
    </div>
  );
}

export function LoadingBoundary({ children }: { children: React.ReactNode }) {
  return (
    <Suspense fallback={<PageLoading />}>
      {children}
    </Suspense>
  );
}