import { NextRequest, NextResponse } from "next/server";

import { getApiBaseUrl, SESSION_COOKIE } from "@/lib/auth";

export async function POST(request: NextRequest) {
  const formData = await request.formData();
  const username = String(formData.get("username") ?? "").trim();
  const password = String(formData.get("password") ?? "").trim();

  const response = await fetch(`${getApiBaseUrl()}/v1/auth/web/login`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ username, password }),
    cache: "no-store",
  }).catch(() => null);

  if (!response || !response.ok) {
    return NextResponse.redirect(new URL("/login?error=1", request.url));
  }

  const payload = (await response.json()) as { session_token: string };
  const redirectResponse = NextResponse.redirect(new URL("/me", request.url));
  redirectResponse.cookies.set(SESSION_COOKIE, payload.session_token, {
    httpOnly: true,
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return redirectResponse;
}
