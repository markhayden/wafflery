import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Installables, bytes } from "../../src/features/workspace/Installables";
import type { Installable } from "../../src/features/actions/model";

test("sizes read as download sizes are quoted", () => {
  expect(bytes(3_400_000_000)).toBe("3.4 GB");
  expect(bytes(120_000_000)).toBe("120 MB");
  expect(bytes(10_867)).toBe("11 kB");
});

const item: Installable = {
  id: "large", title: "Large model", kind: "model", download_bytes: 3_400_000_000, installed_bytes: 3_400_000_000,
  licence: "Example Model License", state: "not_installed", requirements: [], last_outcome: null,
  "x-sample-lab": { lab: { remove: 0.777, best_for: "whole objects" } },
};
const noop = async () => undefined;
test("each installable shows its lab line, size, licence and the action its state allows", () => {
  const html = renderToStaticMarkup(<Installables installables={[item]} onInstall={noop} onUninstall={noop} onRefresh={noop} />);
  expect(html).toContain("Best for whole objects · lab remove score 0.777");
  expect(html).toContain("3.4 GB download · Example Model License");
  expect(html).toContain(">Get<");
  const installing = renderToStaticMarkup(<Installables installables={[{ ...item, state: "installing", progress: 0.4 }]} onInstall={noop} onUninstall={noop} onRefresh={noop} />);
  expect(installing).toContain('aria-valuenow="40"');
  expect(installing).toContain(">Cancel<");
  const installed = renderToStaticMarkup(<Installables installables={[{ ...item, state: "installed" }]} onInstall={noop} onUninstall={noop} onRefresh={noop} />);
  expect(installed).toContain(">Remove<");
});

test("a failed or cancelled last outcome is said; a success is not", () => {
  const failed = renderToStaticMarkup(<Installables installables={[{ ...item, last_outcome: { action: "install", state: "failed", at: "2026-11-09T12:00:00Z", error: { code: "failed", message: "digest mismatch" } } }]} onInstall={noop} onUninstall={noop} onRefresh={noop} />);
  expect(failed).toContain("The last install failed: digest mismatch");
  const succeeded = renderToStaticMarkup(<Installables installables={[{ ...item, state: "installed", last_outcome: { action: "install", state: "succeeded", at: "2026-11-09T12:00:00Z", error: null } }]} onInstall={noop} onUninstall={noop} onRefresh={noop} />);
  expect(succeeded).not.toContain("The last install");
});
