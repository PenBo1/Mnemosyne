/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SidebarSessionList - 侧边栏工作区会话列表组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useCallback } from "react";
import { MessageSquareIcon, Trash2Icon } from "lucide-react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useI18n } from "@/locales/i18n";
import { useAgentStore } from "@/features/chat/store";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import {
  SidebarMenuAction,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 格式化相对时间
 */
function formatRelativeTime(iso: string, locale: string, agentChat: { justNow: string; minutesAgo: string; hoursAgo: string; daysAgo: string; yesterday: string }): string {
  const now = Date.now();
  const then = new Date(iso).getTime();
  const diff = Math.max(0, now - then);
  const min = 60 * 1000;
  const hour = 60 * min;
  const day = 24 * hour;

  if (diff < min) return agentChat.justNow;
  if (diff < hour) {
    const m = Math.floor(diff / min);
    return agentChat.minutesAgo.replace("{count}", String(m));
  }
  if (diff < day) {
    const h = Math.floor(diff / hour);
    return agentChat.hoursAgo.replace("{count}", String(h));
  }
  if (diff < 2 * day) return agentChat.yesterday;
  if (diff < 7 * day) {
    const d = Math.floor(diff / day);
    return agentChat.daysAgo.replace("{count}", String(d));
  }
  return new Date(then).toLocaleDateString(locale === "zh" ? "zh-CN" : "en-US");
}

// ── 可排序会话条目组件 ──────────────────────────────────────────────────────

interface SortableSessionItemProps {
  id: string;
  title: string;
  updatedAt: string;
  isActive: boolean;
  locale: string;
  agentChat: { justNow: string; minutesAgo: string; hoursAgo: string; daysAgo: string; yesterday: string };
  untitledSession: string;
  onSwitch: () => void;
  onDelete: () => void;
}

function SortableSessionItem({
  id,
  title,
  updatedAt,
  isActive,
  locale,
  agentChat,
  untitledSession,
  onSwitch,
  onDelete,
}: SortableSessionItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <SidebarMenuSubItem ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <SidebarMenuSubButton isActive={isActive} onClick={onSwitch}>
        <MessageSquareIcon />
        <span className="flex-1 truncate">
          {title || untitledSession}
        </span>
        <span className="text-xs text-muted-foreground">
          {formatRelativeTime(updatedAt, locale, agentChat)}
        </span>
      </SidebarMenuSubButton>
      <SidebarMenuAction showOnHover onClick={(e) => { e.stopPropagation(); onDelete(); }}>
        <Trash2Icon className="size-4" />
      </SidebarMenuAction>
    </SidebarMenuSubItem>
  );
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 侧边栏工作区会话列表，根据 workspaceId 加载并展示该工作区下的会话
 */
export function SidebarSessionList({ workspaceId }: { workspaceId: string }) {
  const { t, locale } = useI18n();
  const sessions = useAgentStore((s) => s.sessions);
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const loadSessions = useAgentStore((s) => s.loadSessions);
  const switchSession = useAgentStore((s) => s.switchSession);
  const deleteSession = useAgentStore((s) => s.deleteSession);
  const updateSessionSortOrder = useAgentStore((s) => s.updateSessionSortOrder);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);

  useEffect(() => {
    if (activeWorkspaceId === workspaceId) {
      void loadSessions(undefined, workspaceId);
    }
  }, [workspaceId, activeWorkspaceId, loadSessions]);

  // DnD sensors
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = sessions.findIndex((s) => s.id === active.id);
      const newIndex = sessions.findIndex((s) => s.id === over.id);

      const newOrder = arrayMove(sessions, oldIndex, newIndex);
      const ids = newOrder.map((s) => s.id);
      void updateSessionSortOrder(ids);
    }
  }, [sessions, updateSessionSortOrder]);

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={sessions.map((s) => s.id)}
        strategy={verticalListSortingStrategy}
      >
        <SidebarMenuSub>
          {sessions.length === 0 ? (
            <SidebarMenuSubItem>
              <SidebarMenuSubButton>
                <MessageSquareIcon />
                <span className="text-muted-foreground">{t.sidebar.noSessions}</span>
              </SidebarMenuSubButton>
            </SidebarMenuSubItem>
          ) : (
            sessions.map((s) => (
              <SortableSessionItem
                key={s.id}
                id={s.id}
                title={s.title || ""}
                updatedAt={s.updated_at}
                isActive={s.id === currentSessionId}
                locale={locale}
                agentChat={t.agentChat}
                untitledSession={t.sidebar.untitledSession}
                onSwitch={() => void switchSession(s.id)}
                onDelete={() => void deleteSession(s.id)}
              />
            ))
          )}
        </SidebarMenuSub>
      </SortableContext>
    </DndContext>
  );
}