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
    <section className="panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">{subtitle}</p>
          <h2>{title}</h2>
        </div>
      </div>
      <div className="metric-list">
        {items.map((item) => (
          <div key={`${item.label}-${item.value}`} className="metric-row">
            <span>{item.label}</span>
            <strong>{item.value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}
