/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ConversationTimeline - 会话时间线导航组件
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 纵向时间轴形态的对话历史导航组件，核心作用是让用户在长对话、多步骤任务中
 * 快速回溯、定位任意一轮对话或任务节点。
 *
 * 特性：
 * - 左侧刻度标尺：带高斯（钟形曲线）距离感应动效
 * - 时间轴节点：按消息分组（用户/助手轮次）
 * - 激活卡片：悬停显示消息摘要卡片（使用 shadcn HoverCard）
 * - 点击定位：点击节点可滚动到对应消息
 */

import { memo, useCallback, useEffect, useRef, useState, useMemo } from "react";
import { cn } from "@/lib/utils";
import { BotIcon, UserIcon, WrenchIcon, ClockIcon } from "lucide-react";
import { HoverCard, HoverCardTrigger, HoverCardContent } from "@/components/ui/hover-card";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Message } from "@/types";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/** 刻度线基础长度（像素） */
const TICK_BASE_LENGTH = 4;
/** 刻度线最大扩展长度（像素） */
const TICK_MAX_LENGTH = 16;
/** 高斯衰减参数：影响范围半径（像素） */
const GAUSSIAN_RADIUS = 80;
/** 高斯衰减参数：标准差 */
const GAUSSIAN_SIGMA = 40;
/** 刻度线间距（像素） */
const TICK_SPACING = 8;
/** 左侧刻度区域宽度（像素） */
const RULER_WIDTH = 24;

// ── 类型定义 ────────────────────────────────────────────────────────────────

/** 时间线节点 */
interface TimelineNode {
  /** 节点唯一标识 */
  id: string;
  /** 消息索引（用于滚动定位） */
  messageIndex: number;
  /** 节点类型 */
  type: "user" | "assistant" | "tool";
  /** 时间戳 */
  timestamp: string;
  /** 消息内容摘要 */
  content: string;
  /** Y 坐标位置（相对于时间线容器） */
  y: number;
}

/** 会话时间线属性 */
interface ConversationTimelineProps {
  /** 消息列表 */
  messages: Message[];
  /** 当前是否正在流式输出 */
  streaming?: boolean;
  /** 点击节点回调 */
  onNodeClick?: (messageIndex: number) => void;
  /** 是否展开显示 */
  expanded?: boolean;
  /** 自定义类名 */
  className?: string;
}

// ── 高斯函数 ────────────────────────────────────────────────────────────────

/**
 * 计算高斯（正态分布）衰减系数
 * @param distance 距离中心的距离
 * @param sigma 标准差
 * @returns 衰减系数 [0, 1]
 */
function gaussianDecay(distance: number, sigma: number = GAUSSIAN_SIGMA): number {
  return Math.exp(-(distance * distance) / (2 * sigma * sigma));
}

/**
 * 计算刻度线的动态长度
 */
function calculateTickLength(
  distance: number,
  baseLength: number = TICK_BASE_LENGTH,
  maxLength: number = TICK_MAX_LENGTH
): number {
  if (distance > GAUSSIAN_RADIUS) return baseLength;
  const decay = gaussianDecay(distance);
  return baseLength + decay * (maxLength - baseLength);
}

/**
 * 计算刻度线的动态亮度（透明度）
 */
function calculateTickOpacity(distance: number): number {
  if (distance > GAUSSIAN_RADIUS) return 0.3;
  const decay = gaussianDecay(distance);
  return 0.3 + decay * 0.7;
}

// ── 左侧刻度标尺组件 ────────────────────────────────────────────────────────

interface TickRulerProps {
  tickCount: number;
  height: number;
  mouseY: number | null;
  isActive: boolean;
}

const TickRuler = memo(function TickRuler({
  tickCount,
  height,
  mouseY,
  isActive,
}: TickRulerProps) {
  const ticks = useMemo(() => {
    if (tickCount <= 0) return [];
    const spacing = height / (tickCount + 1);
    return Array.from({ length: tickCount }, (_, i) => ({
      index: i,
      y: spacing * (i + 1),
    }));
  }, [tickCount, height]);

  return (
    <div
      className="relative flex items-center justify-end overflow-hidden"
      style={{ width: RULER_WIDTH, height }}
    >
      {ticks.map((tick) => {
        const distance = mouseY !== null ? Math.abs(tick.y - mouseY) : Infinity;
        const length = isActive && mouseY !== null
          ? calculateTickLength(distance)
          : TICK_BASE_LENGTH;
        const opacity = isActive && mouseY !== null
          ? calculateTickOpacity(distance)
          : 0.3;

        return (
          <div
            key={tick.index}
            className="absolute right-0 h-px bg-muted-foreground rounded-full transition-all duration-75 ease-out"
            style={{
              top: tick.y,
              width: length,
              opacity,
              transform: "translateY(-50%)",
            }}
          />
        );
      })}
    </div>
  );
});

// ── 时间轴节点组件（带 HoverCard） ────────────────────────────────────────────

interface TimelineNodeItemProps {
  node: TimelineNode;
  isActive: boolean;
  onClick: () => void;
}

const TimelineNodeItem = memo(function TimelineNodeItem({
  node,
  isActive,
  onClick,
}: TimelineNodeItemProps) {
  const Icon = node.type === "user" ? UserIcon : node.type === "tool" ? WrenchIcon : BotIcon;
  const typeLabel = node.type === "user" ? "用户" : node.type === "tool" ? "工具" : "助手";
  const typeColor = node.type === "user" ? "bg-blue-500/10 text-blue-600 border-blue-500/30"
    : node.type === "tool" ? "bg-amber-500/10 text-amber-600 border-amber-500/30"
    : "bg-emerald-500/10 text-emerald-600 border-emerald-500/30";

  // 格式化时间
  const formattedTime = useMemo(() => {
    try {
      const date = new Date(node.timestamp);
      return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
    } catch {
      return "";
    }
  }, [node.timestamp]);

  return (
    <HoverCard openDelay={100} closeDelay={100}>
      <HoverCardTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className={cn(
            "absolute left-0 size-5 rounded-full border-2 bg-background p-0 transition-all duration-200",
            "hover:scale-125 hover:shadow-md focus:scale-125 focus:shadow-md",
            isActive && "scale-125 shadow-md",
            node.type === "user" && "border-blue-500/50 hover:border-blue-500",
            node.type === "assistant" && "border-emerald-500/50 hover:border-emerald-500",
            node.type === "tool" && "border-amber-500/50 hover:border-amber-500"
          )}
          style={{ top: node.y, transform: "translateY(-50%)" }}
          onClick={onClick}
        >
          <Icon
            className={cn(
              "size-3",
              node.type === "user" && "text-blue-500",
              node.type === "assistant" && "text-emerald-500",
              node.type === "tool" && "text-amber-500"
            )}
          />
        </Button>
      </HoverCardTrigger>

      <HoverCardContent
        side="right"
        align="start"
        sideOffset={8}
        className="w-72 p-0"
      >
        <Card size="sm" className="border-0 shadow-none">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <Badge variant="outline" className={cn("text-xs", typeColor)}>
                <Icon className="size-3 mr-1" />
                {typeLabel}
              </Badge>
              {formattedTime && (
                <span className="text-xs text-muted-foreground flex items-center gap-1">
                  <ClockIcon className="size-3" />
                  {formattedTime}
                </span>
              )}
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            <p className="text-sm text-foreground/90 line-clamp-3 whitespace-pre-wrap break-words">
              {node.content || "（空消息）"}
            </p>
          </CardContent>
        </Card>
      </HoverCardContent>
    </HoverCard>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

export const ConversationTimeline = memo(function ConversationTimeline({
  messages,
  streaming,
  onNodeClick,
  expanded = true,
  className,
}: ConversationTimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [mouseY, setMouseY] = useState<number | null>(null);
  const [isActive, setIsActive] = useState(false);
  const [containerHeight, setContainerHeight] = useState(0);
  const [activeNodeId, setActiveNodeId] = useState<string | null>(null);

  // 监听容器尺寸变化
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerHeight(entry.contentRect.height);
      }
    });

    observer.observe(container);
    setContainerHeight(container.offsetHeight);

    return () => observer.disconnect();
  }, []);

  // 根据消息生成时间线节点
  const nodes = useMemo<TimelineNode[]>(() => {
    if (messages.length === 0) return [];
    if (containerHeight === 0) return [];

    const result: TimelineNode[] = [];
    let lastRole: string | null = null;
    const padding = 16;
    const usableHeight = containerHeight - padding * 2;

    messages.forEach((msg, idx) => {
      if (msg.role === "system") return;

      // 角色变化时创建新节点
      if (msg.role !== lastRole) {
        const y = padding + (idx / messages.length) * usableHeight;
        result.push({
          id: msg.id,
          messageIndex: idx,
          type: msg.role === "user" ? "user" : msg.role === "tool" ? "tool" : "assistant",
          timestamp: msg.created_at,
          content: msg.content.slice(0, 200),
          y,
        });
        lastRole = msg.role;
      }
    });

    return result;
  }, [messages, containerHeight]);

  // 计算刻度线数量
  const tickCount = useMemo(() => {
    return Math.floor(containerHeight / TICK_SPACING);
  }, [containerHeight]);

  // 鼠标事件处理
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setMouseY(e.clientY - rect.top);
  }, []);

  const handleMouseEnter = useCallback(() => {
    setIsActive(true);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setIsActive(false);
    setMouseY(null);
    setActiveNodeId(null);
  }, []);

  // 节点点击处理
  const handleNodeClick = useCallback((node: TimelineNode) => {
    setActiveNodeId(node.id);
    onNodeClick?.(node.messageIndex);
  }, [onNodeClick]);

  if (!expanded) return null;

  return (
    <div
      ref={containerRef}
      className={cn("relative flex h-full select-none", className)}
      onMouseMove={handleMouseMove}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* 左侧刻度标尺 */}
      <TickRuler
        tickCount={tickCount}
        height={containerHeight}
        mouseY={mouseY}
        isActive={isActive}
      />

      {/* 时间轴区域 */}
      <div className="relative flex-1 min-w-0">
        {/* 中轴线 */}
        <div className="absolute left-2.5 top-0 bottom-0 w-px bg-border" />

        {/* 时间线节点 */}
        {nodes.map((node) => (
          <TimelineNodeItem
            key={node.id}
            node={node}
            isActive={activeNodeId === node.id}
            onClick={() => handleNodeClick(node)}
          />
        ))}

        {/* 流式输出指示器 */}
        {streaming && (
          <div className="absolute left-0 bottom-4">
            <Button
              variant="outline"
              size="icon"
              className="size-5 rounded-full border-2 border-primary bg-primary/10 animate-pulse"
            >
              <BotIcon className="size-3 text-primary" />
            </Button>
          </div>
        )}
      </div>
    </div>
  );
});

export default ConversationTimeline;