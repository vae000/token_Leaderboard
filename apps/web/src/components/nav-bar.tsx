"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type CurrentUser = {
  user_id: string;
  display_name: string;
  expires_at: string;
} | null;

const ctaClassName =
  "inline-flex items-center justify-center rounded-full px-4 py-2.5 text-sm font-medium tracking-[0.02em] transition duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-300/60 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_16px_28px_rgba(148,163,184,0.14)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))]";

export function NavBar({ currentUser }: { currentUser: CurrentUser }) {
  const pathname = usePathname();

  const items = [
    { href: "/", label: "仪表盘" },
    { href: "/leaderboards", label: "排行榜" },
    { href: "/me", label: "我的" },
  ];

  return (
    <header className="nav-shell">
      <div className="nav">
        <Link className="nav-brand" href="/">
          <Badge className="w-fit">AI 指挥中心</Badge>
          <div className="nav-brand-row">
            <div className="brand-cluster" aria-hidden="true">
              <span className="brand-grid" />
              <span className="brand-ring brand-ring-outer" />
              <span className="brand-ring brand-ring-inner" />
              <span className="brand-core" />
              <span className="brand-beam" />
            </div>
            <div className="nav-brand-copy">
              <span>Token 榜单</span>
              <strong>企业级 AI 使用驾驶舱</strong>
            </div>
          </div>
        </Link>
        <nav className="nav-links">
          {items.map((item) => (
            <Link
              key={item.href}
              className={cn(pathname === item.href ? "nav-link-active" : "nav-link")}
              href={item.href}
            >
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="flex items-center gap-3">
          {currentUser ? (
            <>
              <span className="text-sm text-slate-600">{currentUser.display_name}</span>
              <form action="/logout" method="POST">
                <button
                  type="submit"
                  className="inline-flex items-center justify-center rounded-full px-4 py-2.5 text-sm font-medium tracking-[0.02em] transition duration-200 border border-sky-100 bg-sky-50/72 text-slate-700 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)] hover:border-sky-200 hover:bg-white/82 hover:text-slate-900"
                >
                  退出
                </button>
              </form>
            </>
          ) : (
            <Link href="/login" className={ctaClassName}>
              登录
            </Link>
          )}
        </div>
      </div>
    </header>
  );
}
