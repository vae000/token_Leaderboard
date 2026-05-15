import { NextRequest, NextResponse } from "next/server";

import { getApiBaseUrl, SESSION_COOKIE } from "@/lib/auth";
import { buildRedirectUrl, isSecureRequest } from "@/lib/request-url";

async function performLogout(request: NextRequest) {
  const sessionToken = request.cookies.get(SESSION_COOKIE)?.value;

  if (sessionToken) {
    await fetch(`${getApiBaseUrl()}/v1/auth/web/logout`, {
      method: "POST",
      headers: { "x-session-token": sessionToken },
      cache: "no-store",
    }).catch(() => null);
  }

  const response = NextResponse.redirect(buildRedirectUrl(request, "/"));
  response.cookies.set(SESSION_COOKIE, "", {
    httpOnly: true,
    sameSite: "lax",
    secure: isSecureRequest(request),
    path: "/",
    maxAge: 0,
  });
  return response;
}

export async function POST(request: NextRequest) {
  return performLogout(request);
}

export async function GET(request: NextRequest) {
  return performLogout(request);
}
