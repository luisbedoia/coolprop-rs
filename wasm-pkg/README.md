# @luisbedoia/coolprop-rs-wasm

Thermophysical properties in the browser: the [CoolProp](https://coolprop.org)
engine compiled to WebAssembly through the `coolprop` Rust crate.

```ts
import { loadCoolProp } from "@luisbedoia/coolprop-rs-wasm";

const cp = await loadCoolProp();          // load once, share the instance
cp.catalog();                             // curated fluids: name, aliases, molar mass, ...

const water = cp.fluid("Water");          // canonical name or alias ("H2O", "R718")
water.critical.temperature;               // 647.096 K, from the equation of state

const s = water.state({ pressure: 101325, temperature: 298.15 });
s.enthalpy;                               // J/kg
s.phase;                                  // "liquid"
```

Everything is SI: Pa, K, kg/m³, J/kg, J/(kg·K).

## States

`fluid.state(inputs)` takes exactly two inputs. The supported pairs are
checked by TypeScript:

- `pressure` with `temperature`, `quality`, `density`, `enthalpy`, `entropy` or `internal_energy`
- `temperature` with `quality`, `density` or `entropy`
- `density` with `quality`, `enthalpy`, `entropy` or `internal_energy`
- `enthalpy` with `entropy`

Properties that do not apply to a state (e.g. `quality` outside the two-phase
region, `cp` inside it) are `null`.

`fluid.states([...])` solves many states in one call. A bad point does not
fail the batch: its entry is the `CoolPropError` for that point.

## Describing the API

Build forms and tables from data instead of hard-coding lists:

```ts
cp.inputs();      // [{ name: "pressure", symbol: "p", unit: "Pa", description, min, max }, ...]
cp.pairs();       // [["pressure", "temperature"], ...]: valid combinations for state()
cp.properties();  // [{ name: "cp", symbol: "cp", unit: "J/(kg·K)", nullable: true, category: "thermodynamic" }, ...]
cp.phases();      // [{ name: "two_phase", label: "two-phase", description: "Liquid-vapor mixture inside the saturation dome" }, ...]
cp.diagrams();    // [{ id: "pressure_enthalpy", x, y, isolines: ["temperature", ...] }, ...]
```

Units are SI and an empty `unit` means dimensionless. Texts are English;
labels and unit conversion are left to the application.

## Diagrams

`fluid.diagram(request)` returns the saturation dome and isolines projected
onto a diagram's axes, ready to plot:

```ts
const d = water.diagram({
  diagram: "pressure_enthalpy",
  isolines: [{ kind: "temperature", count: 7, unit: { scale: 1, offset: -273.15 } }],
});
d.dome.liquid;    // { x: [...], y: [...] }, SI
d.isolines;       // [{ kind: "temperature", value, x, y }, ...]
```

With `count`, isoline values are suggested: round in the given `unit`
(here °C) and spread across the dome. `values` sets them explicitly. Points
CoolProp cannot solve are `null`, a break in the curve. `points` and
`dome_points` set the resolution; the work per request is bounded (at most
30 isolines and 5000 points across them), past that it throws
`invalid_input`.

## Errors

Failures throw `CoolPropError` with a `kind`:

- `unknown_fluid`: the name matches no curated fluid or alias.
- `invalid_input`: malformed request, e.g. an unsupported input pair.
- `coolprop`: CoolProp could not solve it, e.g. a state outside the EOS range.

## Without blocking the UI

`loadCoolProp` solves synchronously on the calling thread. For interactive
pages use `loadCoolPropWorker`: the same API, solved in a Web Worker that
ships with the package, returning promises.

```ts
import { loadCoolPropWorker } from "@luisbedoia/coolprop-rs-wasm";

const cp = await loadCoolPropWorker();
cp.catalog();                              // catalogs and schema stay synchronous
const water = await cp.fluid("Water");
const s = await water.state({ pressure: 101325, temperature: 298.15 });
const batch = await water.states(points);  // the main thread keeps running
cp.terminate();                            // pending and later calls reject
```

The worker is created with `new Worker(new URL("./worker.js", import.meta.url))`,
the pattern bundlers recognize; Vite bundles it (and the `.wasm`) with no
configuration. If your setup cannot resolve it, create a worker from the
package's `./worker` entry yourself and pass it in. With Vite:

```ts
import CoolPropWorker from "@luisbedoia/coolprop-rs-wasm/worker?worker";

const cp = await loadCoolPropWorker({ worker: new CoolPropWorker() });
```

Calls already running in the worker cannot be interrupted; `terminate()`
stops the worker and rejects whatever was pending.

## Building from source

The `dist/` artifacts (`coolprop.js` + `coolprop.wasm`) are built from the
repository root:

```sh
docker compose run --rm build-npm   # wasm artifacts into wasm-pkg/dist/
docker compose run --rm test-npm    # typecheck + browser tests (Chromium)
```
