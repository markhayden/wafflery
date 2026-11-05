import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
import { readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
test("history reports missing saved image bytes without changing the recorded outcome", async ({
  page,
  host,
}) => {
  await host.open(page);
  await prepareSampleInput(page, "Missing image check");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  const run = JSON.parse(
    await readFile(resolve(host.root, "../tool/run-0001/run.json"), "utf8"),
  );
  const hash = run.images.out[0].image.file_sha256 as string;
  await rm(join(host.root, "Library/assets/sha256", hash.slice(0, 2), hash));
  const address = new URL(page.url()).pathname;
  await host.stop();
  await host.start();
  await host.open(page, address);
  await expect(
    page.getByText("Image unavailable. The saved bytes could not be read."),
  ).toBeVisible();
  await expect(
    page.getByRole("status").filter({ hasText: "completed" }),
  ).toBeVisible();
});
