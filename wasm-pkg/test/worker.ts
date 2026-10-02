// Module worker used by coolprop.test.ts: proves the module loads off the
// main thread (Emscripten ENVIRONMENT must include `worker`).
import { loadCoolProp } from "../src/index.js";

self.onmessage = async () => {
  try {
    const cp = await loadCoolProp();
    const s = cp.fluid("Water").state({ pressure: 101325, temperature: 298.15 });
    self.postMessage({ enthalpy: s.enthalpy });
  } catch (e) {
    self.postMessage({ error: String(e) });
  }
};
