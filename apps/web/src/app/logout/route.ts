import { NextRequest, NextResponse } from "next/server";

import { getApiBaseUrl, SESSION_COOKIE } from "@/lib/auth";

const clearCookieOptions = {
  httpOnly: true,
  sameSite: "lax" as const,
  secure: process.env.NODE_ENV === "production",
  path: "/",
  maxAge: 0,
};

async function performLogout(request: NextRequest) {
  const sessionToken = request.cookies.get(SESSION_COOKIE)?.value;

  if (sessionToken) {
    await fetch(`${getApiBaseUrl()}/v1/auth/web/logout`, {
      method: "POST",
      headers: { "x-session-token": sessionToken },
      cache: "no-store",
    }).catch(() => null);
  }

  const response = NextResponse.redirect(new URL("/", request.url));
  response.cookies.set(SESSION_COOKIE, "", clearCookieOptions);
  return response;
}

export async function POST(request: NextRequest) {
  return performLogout(request);
}

export async function GET(request: NextRequest) {
  return performLogout(request);
}
