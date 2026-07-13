import { useCallback, useEffect, useState, type RefObject } from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface UserMessageRef {
  id: string;
  content: string;
}

interface Tick {
  id: string;
  top: number;
  preview: string;
}

function makePreview(content: string): string {
  const trimmed = content.trim().replace(/\s+/g, " ");
  return trimmed.length > 100 ? trimmed.slice(0, 100) + "..." : trimmed;
}

/**
 * 右侧消息导航时间轴：一条居中竖线 + 每条 user message 一个刻度。
 * - hover 刻度：左侧弹出气泡显示该消息预览
 * - 点击刻度：平滑滚动到对应消息
 * - 当前视口最接近的刻度高亮
 */
export function MessageTimeline({
  userMessages,
  scrollRef,
}: {
  userMessages: UserMessageRef[];
  scrollRef: RefObject<HTMLDivElement | null>;
}) {
  const [ticks, setTicks] = useState<Tick[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);

  const updateTicks = useCallback(() => {
    const container = scrollRef.current;
    if (!container) {
      setTicks([]);
      return;
    }
    const scrollHeight = container.scrollHeight;
    if (scrollHeight === 0) {
      setTicks([]);
      return;
    }
    const containerTop = container.getBoundingClientRect().top;
    const newTicks: Tick[] = [];
    for (const msg of userMessages) {
      const el = container.querySelector(`[data-user-message-id="${msg.id}"]`);
      if (!el) continue;
      const elTop =
        el.getBoundingClientRect().top - containerTop + container.scrollTop;
      const percentage = Math.min(
        98,
        Math.max(2, (elTop / scrollHeight) * 100)
      );
      newTicks.push({
        id: msg.id,
        top: percentage,
        preview: makePreview(msg.content),
      });
    }
    setTicks(newTicks);
  }, [userMessages, scrollRef]);

  const updateActive = useCallback(() => {
    const container = scrollRef.current;
    if (!container) return;
    const containerTop = container.getBoundingClientRect().top;
    let closest: { id: string; dist: number } | null = null;
    for (const msg of userMessages) {
      const el = container.querySelector(`[data-user-message-id="${msg.id}"]`);
      if (!el) continue;
      const dist = Math.abs(
        el.getBoundingClientRect().top - containerTop - 24
      );
      if (!closest || dist < closest.dist) {
        closest = { id: msg.id, dist };
      }
    }
    setActiveId(closest?.id ?? null);
  }, [userMessages, scrollRef]);

  useEffect(() => {
    updateTicks();
    updateActive();
    const container = scrollRef.current;
    if (!container) return;
    const onScroll = () => {
      updateTicks();
      updateActive();
    };
    container.addEventListener("scroll", onScroll, { passive: true });
    const observer = new ResizeObserver(() => {
      updateTicks();
      updateActive();
    });
    observer.observe(container);
    return () => {
      container.removeEventListener("scroll", onScroll);
      observer.disconnect();
    };
  }, [updateTicks, updateActive, scrollRef]);

  const handleClick = (id: string) => {
    const container = scrollRef.current;
    if (!container) return;
    const el = container.querySelector(`[data-user-message-id="${id}"]`);
    if (el instanceof HTMLElement) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  };

  if (userMessages.length === 0 || ticks.length === 0) return null;

  return (
    <div className="relative h-full w-10 shrink-0">
      {/* 居中竖线 */}
      <div className="absolute bottom-2 left-1/2 top-2 w-px -translate-x-1/2 bg-border" />
      {/* 刻度 */}
      {ticks.map((tick) => {
        const isActive = activeId === tick.id;
        return (
          <Tooltip key={tick.id}>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => handleClick(tick.id)}
                className={`absolute left-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border transition-all ${
                  isActive
                    ? "scale-125 border-primary bg-primary"
                    : "border-border bg-background hover:border-primary hover:bg-primary/30"
                }`}
                style={{ top: `${tick.top}%` }}
                aria-label="Jump to message"
              />
            </TooltipTrigger>
            <TooltipContent side="left" className="max-w-xs p-2">
              <p className="line-clamp-4 text-xs leading-relaxed text-foreground">
                {tick.preview}
              </p>
            </TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
}
