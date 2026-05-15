import { NextRequest, NextResponse } from "next/server";

import { getApiBaseUrl, SESSION_COOKIE } from "@/lib/auth";
import { buildRedirectUrl, isSecureRequest } from "@/lib/request-url";

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
    return NextResponse.redirect(buildRedirectUrl(request, "/login?error=1"));
  }

  const payload = (await response.json()) as { session_token: string };
  const redirectResponse = NextResponse.redirect(buildRedirectUrl(request, "/me"));
  redirectResponse.cookies.set(SESSION_COOKIE, payload.session_token, {
    httpOnly: true,
    sameSite: "lax",
    secure: isSecureRequest(request),
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return redirectResponse;
}
