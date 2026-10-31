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
    if (response.status === 401) throw new Error("Your workspace session ended. Run wafflehouse to reopen it.");
    if (!response.ok) throw new Error("The workspace could not complete that request.");
    return response;
  }
}
