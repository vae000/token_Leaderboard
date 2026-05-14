import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type SectionCardProps = {
  title: string;
  subtitle: string;
  items: Array<{
    label: string;
    value: string;
  }>;
};

export function SectionCard({ title, subtitle, items }: SectionCardProps) {
  return (
    <Card>
      <CardHeader>
        <div>
          <CardDescription>{subtitle}</CardDescription>
          <CardTitle>{title}</CardTitle>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {items.map((item) => (
          <div
            key={`${item.label}-${item.value}`}
            className="flex items-center justify-between gap-4 rounded-2xl border border-sky-100 bg-white/65 px-4 py-3"
          >
            <span className="text-sm text-slate-600">{item.label}</span>
            <strong className="text-sm font-semibold text-slate-900">{item.value}</strong>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
