import type { NextRequest } from "next/server";

function firstHeaderValue(value: string | null): string | null {
  if (!value) {
    return null;
  }
  return value
    .split(",")
    .map((item) => item.trim())
    .find(Boolean) ?? null;
}

export function buildRedirectUrl(request: NextRequest, pathname: string): URL {
  const forwardedHost = firstHeaderValue(request.headers.get("x-forwarded-host"));
  const host = forwardedHost ?? firstHeaderValue(request.headers.get("host"));
  const protocol = getRequestProtocol(request);

  if (host) {
    return new URL(pathname, `${protocol}://${host}`);
  }

  return new URL(pathname, request.url);
}

export function getRequestProtocol(request: NextRequest): string {
  return firstHeaderValue(request.headers.get("x-forwarded-proto"))
    ?? request.nextUrl.protocol.replace(/:$/, "");
}

export function isSecureRequest(request: NextRequest): boolean {
  return getRequestProtocol(request) === "https";
}
