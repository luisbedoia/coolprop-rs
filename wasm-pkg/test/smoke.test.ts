import { expect, it } from "vitest";

// Placeholder that keeps the browser pipeline exercised until the wasm suite
// is ported.
it("runs in a browser", () => {
  expect(typeof window).toBe("object");
});
