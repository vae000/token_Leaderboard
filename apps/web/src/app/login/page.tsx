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
      <section className="page-heading">
        <p className="eyebrow">登录</p>
        <h1>账号密码登录</h1>
        <p>系统默认使用 `user_id` 作为账号，密码由服务端自动生成。登录成功后会写入当前浏览器会话。</p>
      </section>

      <form
        action="/auth/login"
        method="post"
        className="mx-auto w-full max-w-md rounded-[28px] border border-sky-100/80 bg-white/78 p-8 shadow-[0_24px_50px_rgba(148,163,184,0.14)] backdrop-blur-2xl"
      >
        <div className="space-y-5">
          <label className="block space-y-2">
            <span className="text-sm font-medium text-slate-700">账号</span>
            <input
              name="username"
              type="text"
              placeholder="请输入 user_id"
              className="w-full rounded-2xl border border-sky-100 bg-white/90 px-4 py-3 text-sm text-slate-900 outline-none transition focus:border-sky-300"
              required
            />
          </label>
          <label className="block space-y-2">
            <span className="text-sm font-medium text-slate-700">密码</span>
            <input
              name="password"
              type="password"
              placeholder="请输入自动生成密码"
              className="w-full rounded-2xl border border-sky-100 bg-white/90 px-4 py-3 text-sm text-slate-900 outline-none transition focus:border-sky-300"
              required
            />
          </label>
          <button
            type="submit"
            className="inline-flex w-full items-center justify-center rounded-full px-4 py-3 text-sm font-medium tracking-[0.02em] transition duration-200 border border-sky-200 bg-[linear-gradient(180deg,rgba(255,255,255,0.96),rgba(230,244,255,0.96))] text-slate-900 shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_16px_28px_rgba(148,163,184,0.14)] hover:bg-[linear-gradient(180deg,rgba(255,255,255,1),rgba(219,234,254,0.98))]"
          >
            登录
          </button>
        </div>
      </form>
    </div>
  );
}
