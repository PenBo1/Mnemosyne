/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ConversationTimeline - 会话时间线导航组件
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 悬浮嵌入在对话页面左侧的时间线导航组件。
 * 左侧刻度线数量 = 消息数量，每条消息对应一条刻度线。
 * 带高斯（钟形曲线）距离感应动效。
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
const TICK_BASE_LENGTH = 3;
/** 刻度线最大扩展长度（像素） */
const TICK_MAX_LENGTH = 12;
/** 高斯衰减参数：影响范围半径（像素） */
const GAUSSIAN_RADIUS = 60;
/** 高斯衰减参数：标准差 */
const GAUSSIAN_SIGMA = 30;
/** 时间线宽度（像素） */
const TIMELINE_WIDTH = 20;
/** 时间线距离左边缘的偏移 */
const TIMELINE_OFFSET = 8;

// ── 类型定义 ────────────────────────────────────────────────────────────────

/** 时间线节点 */
interface TimelineNode {
  /** 消息 ID */
  id: string;
  /** 消息索引 */
  messageIndex: number;
  /** 节点类型 */
  type: "user" | "assistant" | "tool";
  /** 时间戳 */
  timestamp: string;
  /** 消息内容摘要 */
  content: string;
  /** Y 坐标位置（相对于消息列表容器） */
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
}

// ── 高斯函数 ────────────────────────────────────────────────────────────────

/**
 * 计算高斯（正态分布）衰减系数
 */
function gaussianDecay(distance: number, sigma: number = GAUSSIAN_SIGMA): number {
  return Math.exp(-(distance * distance) / (2 * sigma * sigma));
}

/**
 * 计算刻度线的动态长度
 */
function calculateTickLength(distance: number): number {
  if (distance > GAUSSIAN_RADIUS) return TICK_BASE_LENGTH;
  const decay = gaussianDecay(distance);
  return TICK_BASE_LENGTH + decay * (TICK_MAX_LENGTH - TICK_BASE_LENGTH);
}

/**
 * 计算刻度线的动态亮度
 */
function calculateTickOpacity(distance: number): number {
  if (distance > GAUSSIAN_RADIUS) return 0.25;
  const decay = gaussianDecay(distance);
  return 0.25 + decay * 0.75;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

export const ConversationTimeline = memo(function ConversationTimeline({
  messages,
  streaming,
  onNodeClick,
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

  // 根据消息生成时间线节点和刻度位置
  const timelineData = useMemo(() => {
    if (messages.length === 0 || containerHeight === 0) {
      return { nodes: [], tickPositions: [] };
    }

    const nodes: TimelineNode[] = [];
    const tickPositions: number[] = [];
    const padding = 12;
    const usableHeight = containerHeight - padding * 2;

    // 过滤掉 system 消息
    const visibleMessages = messages.filter((msg) => msg.role !== "system");

    visibleMessages.forEach((msg, idx) => {
      const y = padding + (idx / Math.max(1, visibleMessages.length - 1)) * usableHeight;
      tickPositions.push(y);

      // 每条消息都创建一个节点
      nodes.push({
        id: msg.id,
        messageIndex: messages.findIndex((m) => m.id === msg.id),
        type: msg.role === "user" ? "user" : msg.role === "tool" ? "tool" : "assistant",
        timestamp: msg.created_at,
        content: msg.content.slice(0, 200),
        y,
      });
    });

    return { nodes, tickPositions };
  }, [messages, containerHeight]);

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

  // 没有消息时不显示
  if (messages.length === 0 && !streaming) {
    return null;
  }

  const { nodes, tickPositions } = timelineData;

  return (
    <div
      ref={containerRef}
      className={cn(
        "fixed left-0 top-0 bottom-0 z-40 pointer-events-none",
        "flex items-stretch"
      )}
      style={{ width: TIMELINE_WIDTH + TIMELINE_OFFSET }}
      onMouseMove={handleMouseMove}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* 刻度标尺区域 */}
      <div
        className="relative flex items-center justify-end pointer-events-auto"
        style={{ width: TIMELINE_OFFSET }}
      >
        {/* 刻度线 - 数量等于消息数量 */}
        {tickPositions.map((y, index) => {
          const distance = mouseY !== null ? Math.abs(y - mouseY) : Infinity;
          const length = isActive && mouseY !== null
            ? calculateTickLength(distance)
            : TICK_BASE_LENGTH;
          const opacity = isActive && mouseY !== null
            ? calculateTickOpacity(distance)
            : 0.25;

          return (
            <div
              key={index}
              className="absolute right-0 h-px bg-muted-foreground rounded-full transition-all duration-75 ease-out"
              style={{
                top: y,
                width: length,
                opacity,
                transform: "translateY(-50%)",
              }}
            />
          );
        })}
      </div>

      {/* 时间线节点区域 */}
      <div className="relative flex-1 pointer-events-auto">
        {/* 中轴线 */}
        <div className="absolute left-1/2 top-3 bottom-3 w-px bg-border/50 -translate-x-1/2" />

        {/* 时间线节点 */}
        {nodes.map((node) => {
          const Icon = node.type === "user" ? UserIcon : node.type === "tool" ? WrenchIcon : BotIcon;
          const typeLabel = node.type === "user" ? "用户" : node.type === "tool" ? "工具" : "助手";
          const typeColor = node.type === "user"
            ? "bg-blue-500/10 text-blue-600 border-blue-500/30"
            : node.type === "tool"
              ? "bg-amber-500/10 text-amber-600 border-amber-500/30"
              : "bg-emerald-500/10 text-emerald-600 border-emerald-500/30";

          const formattedTime = useMemo(() => {
            try {
              const date = new Date(node.timestamp);
              return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
            } catch {
              return "";
            }
          }, [node.timestamp]);

          return (
            <HoverCard key={node.id} openDelay={150} closeDelay={100}>
              <HoverCardTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className={cn(
                    "absolute left-1/2 size-4 rounded-full border p-0 bg-background transition-all duration-200",
                    "hover:scale-150 hover:shadow-sm focus:scale-150 focus:shadow-sm",
                    activeNodeId === node.id && "scale-150 shadow-sm",
                    node.type === "user" && "border-blue-500/50 hover:border-blue-500",
                    node.type === "assistant" && "border-emerald-500/50 hover:border-emerald-500",
                    node.type === "tool" && "border-amber-500/50 hover:border-amber-500"
                  )}
                  style={{ top: node.y, transform: "translate(-50%, -50%)" }}
                  onClick={() => handleNodeClick(node)}
                >
                  <Icon
                    className={cn(
                      "size-2.5",
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
                className="w-64 p-0"
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
        })}

        {/* 流式输出指示器 */}
        {streaming && (
          <div className="absolute left-1/2 bottom-4 -translate-x-1/2">
            <div className="size-4 rounded-full border-2 border-primary bg-primary/10 animate-pulse flex items-center justify-center">
              <BotIcon className="size-2 text-primary" />
            </div>
          </div>
        )}
      </div>
    </div>
  );
});

export default ConversationTimeline;