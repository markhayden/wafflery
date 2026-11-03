import { test, expect } from "./fixtures";
import {visualPolicy} from "./visual-policy";

test("multiple images can be reordered with the keyboard", async ({ page }) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--multiple&viewMode=story",
  );
  const earlier = page.getByRole("button", { name: "Move image 2 earlier" });
  await earlier.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("listitem").first()).toContainText("320 × 100");
  await expect(
    page.getByRole("button", { name: "Move image 1 earlier" }),
  ).toBeDisabled();
});

test("long image lists and labels fit narrow layouts", async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 320, height: 900 });
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--long-list&viewMode=story",
  );
  await expect(page.getByRole("img")).toHaveCount(8);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
  await page.screenshot({
    path: testInfo.outputPath("image-input-long.png"),
    fullPage: true,
  });
});

test("upload failures are visible and preserve an editable input", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--upload-error&viewMode=story",
  );
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByRole("alert")).toHaveText(
    "Not enough disk space to save this image.",
  );
  await expect(page.locator("input[type=file]")).toBeEnabled();
});

test("header dimensions and progress appear before original validation completes",async({page})=>{
 await page.setViewportSize({width:390,height:850});
 await page.goto("http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--pending-import&viewMode=story");
 await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
 await expect(page.getByRole("status")).toContainText("160 × 100");
 await expect(page.getByRole("status")).toContainText("estimated decoded RGBA8");
 await expect(page.getByRole("progressbar",{name:"Importing original image"})).toBeVisible();
 await expect(page.getByRole("img")).toHaveCount(0);
 await expect(page.getByRole("region",{name:"Reference image",exact:true})).toHaveScreenshot("image-import-pending-390.png",{...visualPolicy,mask:[page.getByRole("progressbar")]});
 await page.getByRole("button",{name:"Finish importing"}).click();
 await expect(page.getByRole("progressbar")).toHaveCount(0);
 await expect(page.getByRole("img")).toHaveCount(1);
});

test("image refusal keeps the tool's own reason and remedy visible",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-runoutcome--image-refused&viewMode=story");
 await expect(page.getByText("Input refused by tool.", {exact:true})).toBeVisible();
 await expect(page.getByRole("region",{name:"Run outcome"})).toContainText("Choose a smaller source or adjust the tool’s image-limit configuration.");
 await expect(page.getByRole("region",{name:"Run outcome"})).toHaveScreenshot("image-refusal.png",visualPolicy);
});
