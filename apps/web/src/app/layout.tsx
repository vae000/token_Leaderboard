import type { Metadata } from "next";
import "./globals.css";
import { NavBar } from "@/components/nav-bar";
import { getCurrentUser } from "@/lib/auth";

export const metadata: Metadata = {
  title: "Token 榜单",
  description: "AI 工具使用仪表盘与排行榜",
};

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const currentUser = await getCurrentUser();

  return (
    <html lang="zh-CN" className="h-full antialiased">
      <body className="min-h-full bg-background text-foreground">
        <div className="page-shell">
          <NavBar currentUser={currentUser} />
          <main className="page-content">{children}</main>
        </div>
      </body>
    </html>
  );
}
