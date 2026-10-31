import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Button } from "../../src/components/Button";

test("ordinary buttons do not accidentally submit forms", () => {
  expect(renderToStaticMarkup(<Button>Inspect</Button>)).toContain('type="button"');
});

test("busy state prevents duplicate activation even when disabled is false", () => {
  const html = renderToStaticMarkup(<Button busy disabled={false}>Running…</Button>);
  expect(html).toContain('disabled=""');
  expect(html).toContain('aria-busy="true"');
});

test("explicit form submission remains available", () => {
  expect(renderToStaticMarkup(<Button type="submit">Save</Button>)).toContain('type="submit"');
});
