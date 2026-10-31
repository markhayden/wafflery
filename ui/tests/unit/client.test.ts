import { expect, test } from "bun:test";
import { LocalApi } from "../../src/api/client";

test("bearer credentials cannot be sent to a viewer or a path outside the host API", async () => {
  const api = new LocalApi("http://127.0.0.1:1234", "test-session");
  for (const path of ["http://127.0.0.1:5678/api/state", "//example.com/api/state", "/api/../viewer", "/api/\\example.com"]) {
    await expect(api.request(path)).rejects.toThrow("Only local workspace API paths");
  }
});
