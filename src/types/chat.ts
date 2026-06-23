import type { Message } from "./index";

// ── Message UI State ─────────────────────────────────────────

export type MessageStatus = "pending" | "streaming" | "complete" | "error";

export interface ChatMessage extends Message {
  status: MessageStatus;
  isOptimistic?: boolean;
}

// ── Message Grouping (Date) ──────────────────────────────────

export interface MessageGroup {
  date: string;
  label: string;
  messages: ChatMessage[];
}

// ── Input History ────────────────────────────────────────────

export interface InputHistoryEntry {
  content: string;
  timestamp: number;
}

export interface InputHistoryState {
  entries: InputHistoryEntry[];
  currentIndex: number;
  maxSize: number;
}

// ── Composer State ───────────────────────────────────────────

export interface ComposerState {
  input: string;
  isSubmitting: boolean;
  isFocused: boolean;
}

// ── Timestamp Format ─────────────────────────────────────────

export type TimestampFormat = "relative" | "absolute" | "smart";

export interface TimestampFormatOptions {
  showTime: boolean;
  showDate: boolean;
  format: TimestampFormat;
}

// ── Chat Container Props ─────────────────────────────────────

export interface ChatContainerProps {
  novelId?: string;
  className?: string;
  autoFocus?: boolean;
}

// ── Composer Props ───────────────────────────────────────────

export interface ChatComposerProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onCancel?: () => void;
  disabled?: boolean;
  streaming?: boolean;
  placeholder?: string;
  maxRows?: number;
  minRows?: number;
  autoFocus?: boolean;
  className?: string;
}

// ── Thread Props ─────────────────────────────────────────────

export interface ThreadMessageListProps {
  messages: ChatMessage[];
  streamingContent?: string;
  isStreaming?: boolean;
  onScrollToBottom?: () => void;
  className?: string;
}

export interface MessageItemProps {
  message: ChatMessage;
  onCopy?: () => void;
  onEdit?: () => void;
  className?: string;
}

// ── Timestamp Props ──────────────────────────────────────────

export interface MessageTimestampProps {
  timestamp: string | Date | number;
  format?: TimestampFormat;
  className?: string;
}

// ── Date Separator Props ─────────────────────────────────────

export interface DateSeparatorProps {
  label: string;
  className?: string;
}