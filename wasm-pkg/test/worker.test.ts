/**
 * `loadCoolPropWorker`: the synchronous API solved in the package's own Web
 * Worker. Requires dist/ (see coolprop.test.ts).
 */
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
  CoolPropError,
  loadCoolProp,
  loadCoolPropWorker,
  type CoolProp,
  type CoolPropWorker,
  type State,
} from "../src/index.js";

let cp: CoolPropWorker;
let sync: CoolProp;

beforeAll(async () => {
  [cp, sync] = await Promise.all([loadCoolPropWorker(), loadCoolProp()]);
});

afterAll(() => cp.terminate());

describe("loadCoolPropWorker", () => {
  it("answers catalogs and schema synchronously, like the sync API", () => {
    expect(cp.version()).toBe(sync.version());
    expect(cp.catalog()).toEqual(sync.catalog());
    expect(cp.inputs()).toEqual(sync.inputs());
    expect(cp.pairs()).toEqual(sync.pairs());
    expect(cp.properties()).toEqual(sync.properties());
    expect(cp.phases()).toEqual(sync.phases());
    expect(cp.plotProperties()).toEqual(sync.plotProperties());
    expect(cp.diagrams()).toEqual(sync.diagrams());
  });

  it("solves the same states as the sync API", async () => {
    const water = await cp.fluid("H2O");
    expect(water.name).toBe("Water");
    expect(water.critical).toEqual(sync.fluid("Water").critical);
    const inputs = { pressure: 101325, temperature: 298.15 } as const;
    expect(await water.state(inputs)).toEqual(sync.fluid("Water").state(inputs));
  });

  it("builds the same diagrams as the sync API", async () => {
    const request = {
      diagram: "temperature_entropy",
      points: 30,
      isolines: [{ kind: "pressure" as const, count: 2 }],
    };
    const water = await cp.fluid("Water");
    expect(await water.diagram(request)).toEqual(sync.fluid("Water").diagram(request));
  });

  it("rejects with classified CoolPropErrors", async () => {
    await expect(cp.fluid("Unobtainium")).rejects.toMatchObject({
      name: "CoolPropError",
      kind: "unknown_fluid",
    });
    const water = await cp.fluid("Water");
    const err = await water.state({ pressure: 101325, temperature: -10 }).catch((e) => e);
    expect(err).toBeInstanceOf(CoolPropError);
    expect(err.kind).toBe("coolprop");
  });

  it("returns per-point CoolPropErrors in batches", async () => {
    const water = await cp.fluid("Water");
    const [ok, bad] = await water.states([
      { pressure: 101325, quality: 0 },
      { pressure: 101325, temperature: -10 },
    ]);
    expect((ok as State).quality).toBe(0);
    expect(bad).toBeInstanceOf(CoolPropError);
    expect((bad as CoolPropError).kind).toBe("coolprop");
  });

  it("matches concurrent requests to their responses", async () => {
    const water = await cp.fluid("Water");
    const temperatures = Array.from({ length: 200 }, (_, i) => 280 + i * 0.25);
    const states = await Promise.all(
      temperatures.map((temperature) => water.state({ pressure: 101325, temperature })),
    );
    states.forEach((s, i) => expect(s.temperature).toBeCloseTo(temperatures[i], 9));
  });

  it("keeps the main thread free while solving", async () => {
    const water = await cp.fluid("Water");
    const points = Array.from({ length: 50_000 }, (_, i) => ({
      pressure: 101325,
      temperature: 280 + (i % 80),
    }));
    let ticks = 0;
    const timer = setInterval(() => ticks++, 5);
    const results = await water.states(points);
    clearInterval(timer);
    expect(results).toHaveLength(points.length);
    // The batch takes over a second in the worker; a blocked main thread
    // would not have run the timer at all.
    expect(ticks).toBeGreaterThan(10);
  });
});

describe("terminate", () => {
  it("rejects pending and later calls", async () => {
    const own = await loadCoolPropWorker();
    const water = await own.fluid("Water");
    const pending = water.states(
      Array.from({ length: 20_000 }, () => ({ pressure: 101325, temperature: 300 })),
    );
    own.terminate();
    await expect(pending).rejects.toThrow("terminated");
    await expect(water.state({ pressure: 101325, temperature: 300 })).rejects.toThrow(
      "terminated",
    );
    await expect(own.fluid("Water")).rejects.toThrow("terminated");
  });
});
