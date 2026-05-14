import "server-only";

import { cookies } from "next/headers";

const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

export const SESSION_COOKIE = "token_session";

export type CurrentUser = {
  user_id: string;
  display_name: string;
  expires_at: string;
};

export async function getSessionToken(): Promise<string | null> {
  return (await cookies()).get(SESSION_COOKIE)?.value ?? null;
}

export async function getCurrentUser(): Promise<CurrentUser | null> {
  const token = await getSessionToken();
  if (!token) {
    return null;
  }

  const response = await fetch(`${API_BASE_URL}/v1/auth/web/me`, {
    cache: "no-store",
    headers: {
      "x-session-token": token,
    },
  }).catch(() => null);

  if (!response || !response.ok) {
    return null;
  }

  const payload = (await response.json()) as {
    user: { user_id: string; display_name: string };
    expires_at: string;
  };

  return {
    user_id: payload.user.user_id,
    display_name: payload.user.display_name,
    expires_at: payload.expires_at,
  };
}

export function getApiBaseUrl(): string {
  return API_BASE_URL;
}
