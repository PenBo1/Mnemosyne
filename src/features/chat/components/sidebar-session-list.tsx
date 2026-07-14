import { useEffect } from "react";
import { MessageSquareIcon, Trash2Icon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useAgentStore } from "@/features/chat/store";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import {
  SidebarMenuAction,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar";

/** 相对时间格式化（如 "刚刚"、"3 分钟前"、"昨天"） */
function formatRelativeTime(iso: string, locale: string): string {
  const now = Date.now();
  const then = new Date(iso).getTime();
  const diff = Math.max(0, now - then);
  const min = 60 * 1000;
  const hour = 60 * min;
  const day = 24 * hour;

  if (diff < min) return locale === "zh" ? "刚刚" : "just now";
  if (diff < hour) {
    const m = Math.floor(diff / min);
    return locale === "zh" ? `${m} 分钟前` : `${m}m ago`;
  }
  if (diff < day) {
    const h = Math.floor(diff / hour);
    return locale === "zh" ? `${h} 小时前` : `${h}h ago`;
  }
  if (diff < 2 * day) return locale === "zh" ? "昨天" : "yesterday";
  if (diff < 7 * day) {
    const d = Math.floor(diff / day);
    return locale === "zh" ? `${d} 天前` : `${d}d ago`;
  }
  return new Date(then).toLocaleDateString(locale === "zh" ? "zh-CN" : "en-US");
}

/**
 * 侧边栏工作区会话列表。
 *
 * - 根据 workspaceId 加载该工作区下的会话
 * - 点击会话项切换；hover 显示删除按钮
 */
export function SidebarSessionList({ workspaceId }: { workspaceId: string }) {
  const { t, locale } = useI18n();
  const sessions = useAgentStore((s) => s.sessions);
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const loadSessions = useAgentStore((s) => s.loadSessions);
  const switchSession = useAgentStore((s) => s.switchSession);
  const deleteSession = useAgentStore((s) => s.deleteSession);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);

  useEffect(() => {
    if (activeWorkspaceId === workspaceId) {
      void loadSessions(undefined, workspaceId);
    }
  }, [workspaceId, activeWorkspaceId, loadSessions]);

  return (
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
          <SidebarMenuSubItem key={s.id}>
            <SidebarMenuSubButton
              isActive={s.id === currentSessionId}
              onClick={() => void switchSession(s.id)}
            >
              <MessageSquareIcon />
              <span className="flex-1 truncate">
                {s.title || t.sidebar.untitledSession}
              </span>
              <span className="text-xs text-muted-foreground">
                {formatRelativeTime(s.updated_at, locale)}
              </span>
            </SidebarMenuSubButton>
            <SidebarMenuAction
              showOnHover
              onClick={(e) => {
                e.stopPropagation();
                void deleteSession(s.id);
              }}
            >
              <Trash2Icon className="size-4" />
            </SidebarMenuAction>
          </SidebarMenuSubItem>
        ))
      )}
    </SidebarMenuSub>
  );
}
