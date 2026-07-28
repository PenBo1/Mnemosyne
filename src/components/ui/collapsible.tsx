/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Collapsible - 折叠面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { Collapsible as CollapsiblePrimitive } from "radix-ui"

// ── 折叠面板组件 ────────────────────────────────────────────────────────────

function Collapsible({
  ...props
}: React.ComponentProps<typeof CollapsiblePrimitive.Root>) {
  return <CollapsiblePrimitive.Root data-slot="collapsible" {...props} />
}

// ── 折叠触发器组件 ──────────────────────────────────────────────────────────

function CollapsibleTrigger({
  ...props
}: React.ComponentProps<typeof CollapsiblePrimitive.CollapsibleTrigger>) {
  return (
    <CollapsiblePrimitive.CollapsibleTrigger
      data-slot="collapsible-trigger"
      {...props}
    />
  )
}

// ── 折叠内容组件 ────────────────────────────────────────────────────────────

function CollapsibleContent({
  ...props
}: React.ComponentProps<typeof CollapsiblePrimitive.CollapsibleContent>) {
  return (
    <CollapsiblePrimitive.CollapsibleContent
      data-slot="collapsible-content"
      {...props}
    />
  )
}

export { Collapsible, CollapsibleTrigger, CollapsibleContent }
