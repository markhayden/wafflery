export class ApiError extends Error {
  constructor(message: string, readonly status: number) { super(message); }
}

/** Only the host origin receives credentials. Tool viewers use separate, uncredentialed URLs. */
export class LocalApi {
  constructor(private readonly origin: string, private readonly token: string) {}

  async request(path: string, init: RequestInit = {}): Promise<Response> {
    const url = new URL(path, this.origin);
    if (!path.startsWith("/api/") || path.includes("\\") || url.origin !== this.origin || !url.pathname.startsWith("/api/")) {
      throw new Error("Only local workspace API paths are allowed.");
    }
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${this.token}`);
    const response = await fetch(url, { ...init, headers, credentials: "omit", mode: "same-origin", redirect: "error" });
    if (response.status === 401) throw new ApiError("Your workspace session ended. Run wafflery to reopen it.", response.status);
    if (!response.ok) {
      const payload: unknown = await response.json().catch(() => null);
      if (payload && typeof payload === "object" && "error" in payload && payload.error && typeof payload.error === "object" && "message" in payload.error && typeof payload.error.message === "string") {
        throw new ApiError(payload.error.message, response.status);
      }
      throw new ApiError("The workspace could not complete that request.", response.status);
    }
    return response;
  }
}
