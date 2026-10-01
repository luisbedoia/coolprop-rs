import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// The wasm module is web-only (Emscripten linked with ENVIRONMENT=web), so its
// tests run in a real browser via Playwright/Chromium, never in node.
// `docker compose run --rm test-npm` mirrors this in CI; locally `npm test`
// uses the host's Playwright browser.
export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    browser: {
      enabled: true,
      provider: playwright(),
      headless: true,
      // A single Chromium instance is enough; the wasm output is engine-agnostic.
      instances: [{ browser: "chromium" }],
    },
  },
});
