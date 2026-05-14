import { redirect } from "next/navigation";

import { getCurrentUser } from "@/lib/auth";

export const dynamic = "force-dynamic";

export default async function LoginPage() {
  const currentUser = await getCurrentUser();
  if (currentUser) {
    redirect("/me");
  }

  return (
    <div className="stack-xl">
      <section className="mx-auto w-full max-w-md pt-16">
        <form
          action="/auth/login"
          method="post"
          className="mt-8 rounded-2xl border border-sky-100/80 bg-white/78 p-6 shadow-[0_16px_32px_rgba(148,163,184,0.12)]"
        >
          <div className="space-y-5">
            <label className="block space-y-2">
              <span className="text-sm font-medium text-slate-700">账号</span>
              <input
                name="username"
                type="text"
                placeholder="账号"
                className="w-full rounded-xl border border-sky-100 bg-white/90 px-4 py-3 text-sm text-slate-900 outline-none transition focus:border-sky-300"
                required
              />
            </label>
            <label className="block space-y-2">
              <span className="text-sm font-medium text-slate-700">密码</span>
              <input
                name="password"
                type="password"
                placeholder="密码"
                className="w-full rounded-xl border border-sky-100 bg-white/90 px-4 py-3 text-sm text-slate-900 outline-none transition focus:border-sky-300"
                required
              />
            </label>
            <button
              type="submit"
              className="inline-flex w-full items-center justify-center rounded-xl px-4 py-3 text-sm font-medium tracking-[0.02em] transition duration-200 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_4px_8px_rgba(148,163,184,0.1)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))]"
            >
              登录
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
