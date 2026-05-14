import { NextRequest, NextResponse } from "next/server";

import { SESSION_COOKIE } from "@/lib/auth";

export async function GET(request: NextRequest) {
  const sessionToken = request.nextUrl.searchParams.get("session_token");
  const next = request.nextUrl.searchParams.get("next");
  const target =
    next && next.startsWith("/") && !next.startsWith("//") ? next : "/me";

  const response = NextResponse.redirect(new URL(target, request.url));
  if (!sessionToken) {
    // Must match the same options used when setting the cookie in auth/login/route.ts
    response.cookies.set(SESSION_COOKIE, "", {
      httpOnly: true,
      sameSite: "lax",
      secure: process.env.NODE_ENV === "production",
      path: "/",
      maxAge: 0,
    });
    return response;
  }

  response.cookies.set(SESSION_COOKIE, sessionToken, {
    httpOnly: true,
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return response;
}
