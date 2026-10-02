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

## Errors

Failures throw `CoolPropError` with a `kind`:

- `unknown_fluid`: the name matches no curated fluid or alias.
- `invalid_input`: malformed request, e.g. an unsupported input pair.
- `coolprop`: CoolProp could not solve it, e.g. a state outside the EOS range.

## Web Workers

The module loads on the main thread and inside module workers. Calls are
synchronous, so run large batches in a worker to keep the UI responsive.

## Building from source

The `dist/` artifacts (`coolprop.js` + `coolprop.wasm`) are built from the
repository root:

```sh
docker compose run --rm build-npm   # wasm artifacts into wasm-pkg/dist/
docker compose run --rm test-npm    # typecheck + browser tests (Chromium)
```
