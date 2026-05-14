type StatCardProps = {
  label: string;
  value: string;
  caption: string;
};

export function StatCard({ label, value, caption }: StatCardProps) {
  return (
    <article className="stat-card">
      <span className="eyebrow">{label}</span>
      <strong>{value}</strong>
      <p>{caption}</p>
    </article>
  );
}
