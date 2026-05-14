import { Card, CardContent } from "@/components/ui/card";

type AccentColor = "blue" | "purple" | "emerald" | "amber" | "rose";

const accentStyles: Record<AccentColor, {
  bar: string;
  label: string;
}> = {
  blue: {
    bar: "via-blue-400/60",
    label: "text-blue-600/80",
  },
  purple: {
    bar: "via-purple-400/60",
    label: "text-purple-600/80",
  },
  emerald: {
    bar: "via-emerald-400/60",
    label: "text-emerald-600/80",
  },
  amber: {
    bar: "via-amber-400/60",
    label: "text-amber-600/80",
  },
  rose: {
    bar: "via-rose-400/60",
    label: "text-rose-600/80",
  },
};

type StatCardProps = {
  label: string;
  value: string;
  caption: string;
  color?: AccentColor;
};

export function StatCard({ label, value, caption, color = "blue" }: StatCardProps) {
  const style = accentStyles[color];

  return (
    <Card className="relative overflow-hidden">
      <div className={`absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent ${style.bar} to-transparent`} />
      <CardContent className="space-y-3">
        <span className={`block font-mono text-[11px] uppercase tracking-[0.22em] ${style.label}`}>
          {label}
        </span>
        <strong className="block text-3xl font-semibold tracking-[-0.05em] text-slate-900">
          {value}
        </strong>
        <p className="max-w-[18rem] text-sm leading-6 text-slate-600">{caption}</p>
      </CardContent>
    </Card>
  );
}
