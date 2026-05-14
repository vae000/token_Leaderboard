"use client";

import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const DEPARTMENTS = [
  { id: "t_data", label: "数据部" },
  { id: "t_eng", label: "研发部" },
  { id: "t_product", label: "产品部" },
  { id: "t_ops", label: "运维部" },
];

type TeamSettingsProps = {
  userId: string;
  currentTeamName?: string | null;
};

export function TeamSettings({ userId, currentTeamName }: TeamSettingsProps) {
  const [selected, setSelected] = useState(
    currentTeamName ? (DEPARTMENTS.find((d) => d.label === currentTeamName)?.id ?? "") : "",
  );
  const [savedDept, setSavedDept] = useState<string | null>(currentTeamName ?? null);
  const [status, setStatus] = useState<"idle" | "saving" | "done" | "error">("idle");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!selected) return;
    setStatus("saving");

    try {
      const dept = DEPARTMENTS.find((d) => d.id === selected)!;
      const { assignUserTeam } = await import("@/lib/api");
      await assignUserTeam(userId, dept.id, dept.label);
      setSavedDept(dept.label);
      setStatus("done");
    } catch {
      setStatus("error");
    }
  }

  return (
    <Card>
      <CardHeader>
        <div>
          <CardDescription>团队归属</CardDescription>
          <CardTitle>所属部门</CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          {savedDept && (
            <div className="flex items-center gap-2 rounded-xl border border-sky-100 bg-sky-50/60 px-4 py-3 text-sm">
              <svg className="h-4 w-4 text-sky-500" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" d="m4.5 12.75 6 6 9-13.5" />
              </svg>
              <span className="text-slate-600">当前部门：</span>
              <span className="font-medium text-slate-900">{savedDept}</span>
            </div>
          )}

          <select
            value={selected}
            onChange={(e) => { setSelected(e.target.value); setStatus("idle"); }}
            className="w-full rounded-xl border border-sky-100 bg-white/90 px-3.5 py-2.5 text-sm text-slate-900 outline-none transition focus:border-sky-300"
            required
          >
            <option value="" disabled>请选择部门</option>
            {DEPARTMENTS.map((dept) => (
              <option key={dept.id} value={dept.id}>
                {dept.label}
              </option>
            ))}
          </select>

          <div className="flex items-center gap-3">
            <button
              type="submit"
              disabled={status === "saving" || !selected}
              className="inline-flex items-center justify-center rounded-xl px-5 py-2.5 text-sm font-medium transition duration-200 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_4px_8px_rgba(148,163,184,0.1)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))] disabled:opacity-50"
            >
              {status === "saving" ? "保存中…" : "保存"}
            </button>

            {status === "done" && (
              <span className="text-sm text-emerald-600">已更新</span>
            )}
            {status === "error" && (
              <span className="text-sm text-rose-500">保存失败，请重试</span>
            )}
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
