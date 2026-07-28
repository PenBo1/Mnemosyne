/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SearchInput - 搜索输入框组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import * as React from "react"
import { memo } from "react"
import { SearchIcon } from "lucide-react"
import { cn } from "@/lib/utils"
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group"

// ── 搜索输入框组件 ──────────────────────────────────────────────────────────

const SearchInput = memo(function SearchInput({
  className,
  icon,
  ...props
}: Omit<React.ComponentProps<"input">, "type"> & {
  icon?: React.ReactNode
}) {
  return (
    <InputGroup className={cn("max-w-sm flex-1", className)}>
      <InputGroupAddon align="inline-start">
        {icon ?? <SearchIcon />}
      </InputGroupAddon>
      <InputGroupInput type="search" {...props} />
    </InputGroup>
  )
})

export { SearchInput }