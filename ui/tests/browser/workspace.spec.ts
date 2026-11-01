import { test, expect } from "./tool-host";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
test("an unavailable launcher selection never silently opens another tool", async ({
  page,
  host,
}) => {
  await host.open(page);
  // Let the one-use bootstrap establish this tab's session before navigating again.
  await expect(page.getByRole("button", { name: "Tool sample-lab", exact: true })).toBeVisible();
  await page.goto(`${host.origin}/?tool=not-installed`);
  await expect(
    page.getByRole("status").filter({ hasText: "not-installed" }),
  ).toContainText("not registered");
  await expect(
    page.getByRole("button", { name: "Run", exact: true }),
  ).toBeDisabled();
  await expect(page.locator("input[type=file]")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Tool Choose tool", exact: true })
    .click();
  await page.getByRole("button", { name: /sample-lab Version/ }).click();
  await expect(
    page.getByRole("button", { name: "Action Add a test band" }),
  ).toBeVisible();
});
test("run a real offline action and inspect immutable evidence after editing the draft", async ({
  page,
  host,
}, testInfo) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await host.open(page);
  await expect(page.locator(".workspace-brand img")).toBeVisible();
  await expect
    .poll(() =>
      page
        .locator(".workspace-brand img")
        .evaluate(
          (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
        ),
    )
    .toBe(true);
  await expect(
    page.getByRole("button", { name: /Action Add a test band/ }),
  ).toBeVisible();
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/sample.png");
  await page
    .getByLabel("Label *", { exact: true })
    .fill("Saved first experiment");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("slider", { name: "Before and after divider" }),
  ).toBeVisible();
  await page.getByLabel("Label *", { exact: true }).fill("Unsubmitted draft");
  await page.getByRole("button", { name: "Inspect run" }).click();
  const drawer = page.getByRole("dialog", { name: "Run details" });
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await expect(drawer.getByLabel("Saved evidence")).not.toContainText(
    "Unsubmitted draft",
  );
  await drawer.getByRole("button", { name: "Recorded call" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "progressToken",
  );
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await drawer.getByRole("button", { name: "Raw result" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "structuredContent",
  );
  await drawer.getByRole("button", { name: "Close" }).click();
  await page.screenshot({
    path: testInfo.outputPath("offline-result.png"),
    fullPage: true,
  });
  await host.stop();
  // An older library record is evidence to inspect, not a live v0.2 tool declaration.
  const legacy = spawnSync("/usr/bin/sqlite3", [join(host.root, "Library", "library.sqlite3"),
    `UPDATE attempts SET record=json_set(json_remove(record,'$.declaration.document.actions[0].readiness'),
      '$.declaration.document.contract','0.2.0',
      '$.declaration.document.actions[0].cost.needs_env',json('[]'),
      '$.result.contract','0.2.0','$.raw_response.structuredContent.contract','0.2.0');`
  ], { encoding: "utf8" });
  expect(legacy.status, legacy.stderr).toBe(0);
  await host.start();
  await host.open(page);
  await expect(
    page.getByRole("slider", { name: "Before and after divider" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Inspect run" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await drawer.getByRole("button", { name: "Close" }).click();
  await page.getByRole("button", { name: "History", exact: true }).click();
  const history = page.getByRole("dialog", { name: "History", exact: true });
  await history.getByRole("button", { name: /sample \/ mark_band/ }).click();
  await expect(page.getByRole("slider")).toBeVisible();
  await page.getByRole("button", { name: "Inspect run" }).click();
  await drawer.getByRole("button", { name: "Raw result" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText('"contract": "0.2.0"');
  await drawer.getByRole("button", { name: "Close" }).click();
  expect(errors).toEqual([]);
});

test("tool discovery does not expose a temporary image form that loses uploads", async ({
  page,
  host,
}) => {
  let requests = 0;
  let release: () => void = () => {};
  const ready = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/state", async (route) => {
    requests++;
    if (requests === 2) await ready;
    await route.continue();
  });
  try {
    await host.open(page);
    await expect(
      page.getByRole("status").filter({ hasText: "Connected locally" }),
    ).toBeVisible();
    await expect(page.getByText("Loading workspace…")).toBeVisible();
    await expect(page.locator("input[type=file]")).toHaveCount(0);
  } finally {
    release();
  }
  await expect(
    page.getByRole("button", { name: /Action Add a test band/ }),
  ).toBeVisible();
  await expect(page.locator("input[type=file]")).toHaveCount(1);
});
