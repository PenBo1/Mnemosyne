import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface StatCardProps {
    label: string;
    value: string | number;
    icon?: React.ReactNode;
    hint?: string;
    className?: string;
}

export function StatCard({ label, value, icon, hint, className }: StatCardProps) {
    return (
        <Card className={cn("p-4", className)}>
            <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                    <p className="trae-eyebrow">{label}</p>
                    <p className="trae-stat-value mt-1.5 truncate">{value}</p>
                    {hint && (
                        <p className="mt-1 text-xs text-[var(--text-tertiary)]">{hint}</p>
                    )}
                </div>
                {icon && (
                    <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-[var(--bg-overlay-l2)] text-[var(--text-secondary)]">
                        {icon}
                    </div>
                )}
            </div>
        </Card>
    );
}
