import { Card, CardContent } from "@/components/ui/card";

type StatCardProps = {
  label: string;
  value: string;
  caption: string;
};

export function StatCard({ label, value, caption }: StatCardProps) {
  return (
    <Card className="relative overflow-hidden">
      <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-sky-200 to-transparent" />
      <CardContent className="space-y-3">
        <span className="font-mono text-[11px] uppercase tracking-[0.22em] text-sky-600/70">
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
