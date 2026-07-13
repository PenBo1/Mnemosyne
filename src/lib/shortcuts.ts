const IS_MAC =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);

export const MOD_PROP: "meta" | "ctrl" = IS_MAC ? "meta" : "ctrl";

export type ShortcutId =
  | "newChat"
  | "send"
  | "closeDialog"
  | "newlineInInput"
  | "submitMessage"
  | "save"
  | "search"
  | "previousInput"
  | "nextInput";

export type ShortcutGroup = "global" | "chat" | "editor" | "navigation";

export interface KeyBinding {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
}

export interface Shortcut {
  id: ShortcutId;
  group: ShortcutGroup;
  defaultBindings: KeyBinding[];
  allowRepeat?: boolean;
}

export const SHORTCUTS: Shortcut[] = [
  { id: "newChat", group: "global", defaultBindings: [{ [MOD_PROP]: true, key: "n" }] },
  { id: "send", group: "global", defaultBindings: [{ [MOD_PROP]: true, key: "Enter" }] },
  { id: "closeDialog", group: "global", defaultBindings: [{ key: "Escape" }] },
  { id: "newlineInInput", group: "chat", defaultBindings: [{ key: "Enter" }] },
  { id: "submitMessage", group: "chat", defaultBindings: [{ [MOD_PROP]: true, key: "Enter" }] },
  { id: "save", group: "editor", defaultBindings: [{ [MOD_PROP]: true, key: "s" }] },
  { id: "search", group: "navigation", defaultBindings: [{ [MOD_PROP]: true, key: "k" }] },
  { id: "previousInput", group: "navigation", defaultBindings: [{ key: "ArrowUp" }] },
  { id: "nextInput", group: "navigation", defaultBindings: [{ key: "ArrowDown" }] },
];

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  "global",
  "chat",
  "editor",
  "navigation",
];

const BY_ID = new Map(SHORTCUTS.map((s) => [s.id, s]));

export function getDefaultBindings(id: ShortcutId): KeyBinding[] {
  return BY_ID.get(id)?.defaultBindings ?? [];
}

export function matchBinding(e: KeyboardEvent, binding: KeyBinding): boolean {
  const eventKey = e.key.toLowerCase();
  const bindingKey = binding.key.toLowerCase();
  if (eventKey !== bindingKey) return false;
  return (
    !!e.ctrlKey === !!binding.ctrl &&
    !!e.shiftKey === !!binding.shift &&
    !!e.altKey === !!binding.alt &&
    !!e.metaKey === !!binding.meta
  );
}

export function bindingsEqual(a: KeyBinding, b: KeyBinding): boolean {
  return (
    a.key.toLowerCase() === b.key.toLowerCase() &&
    !!a.ctrl === !!b.ctrl &&
    !!a.shift === !!b.shift &&
    !!a.alt === !!b.alt &&
    !!a.meta === !!b.meta
  );
}

export function getBindingTokens(binding?: KeyBinding): string[] {
  if (!binding) return [];
  const tokens: string[] = [];
  if (IS_MAC) {
    if (binding.ctrl) tokens.push("⌃");
    if (binding.alt) tokens.push("⌥");
    if (binding.shift) tokens.push("⇧");
    if (binding.meta) tokens.push("⌘");
  } else {
    if (binding.ctrl) tokens.push("Ctrl");
    if (binding.alt) tokens.push("Alt");
    if (binding.shift) tokens.push("Shift");
    if (binding.meta) tokens.push("Win");
  }

  let keyLabel = binding.key;
  if (keyLabel === " ") keyLabel = "Space";
  else if (keyLabel === "ArrowUp") keyLabel = "↑";
  else if (keyLabel === "ArrowDown") keyLabel = "↓";
  else if (keyLabel === "ArrowLeft") keyLabel = "←";
  else if (keyLabel === "ArrowRight") keyLabel = "→";
  else if (keyLabel === "Escape") keyLabel = "Esc";
  else if (keyLabel.length === 1) keyLabel = keyLabel.toUpperCase();

  tokens.push(keyLabel);
  return tokens;
}