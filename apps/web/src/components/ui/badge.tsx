import * as React from "react";

import { cn } from "@/lib/utils";

type BadgeProps = React.HTMLAttributes<HTMLDivElement> & {
  variant?: "default" | "outline";
};

export function Badge({ className, variant = "default", ...props }: BadgeProps) {
  return (
    <div
      className={cn(
        "inline-flex items-center rounded-full border px-3 py-1 text-[11px] font-medium uppercase tracking-[0.24em]",
        variant === "default"
          ? "border-sky-200 bg-white/76 text-sky-700"
          : "border-sky-100/90 bg-sky-50/70 text-slate-500",
        className,
      )}
      {...props}
    />
  );
}
