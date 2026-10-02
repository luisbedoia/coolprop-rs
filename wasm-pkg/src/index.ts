/**
 * Typed facade over the coolprop wasm module.
 *
 *   import { loadCoolProp } from "@luisbedoia/coolprop-rs-wasm";
 *   const cp = await loadCoolProp();
 *   const water = cp.fluid("Water");            // or an alias: "H2O"
 *   const s = water.state({ pressure: 101325, temperature: 298.15 });
 *   s.enthalpy;                                 // J/kg
 *
 * Calls are synchronous. To keep a UI responsive, use `loadCoolPropWorker`
 * instead: same API, solved in a Web Worker, returning promises.
 */

import type {
  CriticalPoint,
  Envelope,
  ErrorBody,
  ErrorKind,
  FluidData,
  InputInfo,
  InputName,
  PhaseInfo,
  PropertyInfo,
  Schema,
  State,
  StateInputs,
} from "./types.js";

export type * from "./types.js";
export { loadCoolPropWorker } from "./worker-client.js";
export type { CoolPropWorker, WorkerFluid, WorkerOptions } from "./worker-client.js";

/** A curated fluid, ready to solve states. */
export interface Fluid {
  /** Canonical CoolProp name. */
  readonly name: string;
  readonly data: FluidData;
  /** Critical point of the equation of state. */
  readonly critical: CriticalPoint;
  /** Solves the state fixed by two inputs. Throws {@link CoolPropError}. */
  state(inputs: StateInputs): State;
  /**
   * Solves many states in one call. Never throws for a single bad point:
   * each entry is either a `State` or the `CoolPropError` for that point.
   */
  states(inputs: readonly StateInputs[]): (State | CoolPropError)[];
}

/** What both the synchronous and the worker APIs know without solving. */
export interface CoolPropInfo {
  /** Underlying CoolProp version, e.g. "8.0.0". */
  version(): string;
  /** Every curated fluid. */
  catalog(): readonly FluidData[];
  /** Inputs that can fix a state, with symbol, unit and bounds. */
  inputs(): readonly InputInfo[];
  /** Pairs of inputs accepted by `fluid.state` (in either order). */
  pairs(): readonly (readonly [InputName, InputName])[];
  /** Numeric properties of a `State`, with symbol, unit and category. */
  properties(): readonly PropertyInfo[];
  /** Phases a `State` can be in. */
  phases(): readonly PhaseInfo[];
}

export interface CoolProp extends CoolPropInfo {
  /** Opens a fluid by canonical name or alias. Throws {@link CoolPropError}. */
  fluid(name: string): Fluid;
}

export class CoolPropError extends Error {
  readonly kind: ErrorKind;
  constructor(body: ErrorBody) {
    super(body.message);
    this.name = "CoolPropError";
    this.kind = body.kind;
  }
}

/** The subset of the Emscripten module the facade uses. */
export interface CoolPropModule {
  _malloc(size: number): number;
  _free(ptr: number): void;
  _coolprop_version(): number;
  _coolprop_catalog(): number;
  _coolprop_schema(): number;
  _coolprop_fluid(request: number): number;
  _coolprop_state(request: number): number;
  _coolprop_states(request: number): number;
  _coolprop_free_string(ptr: number): void;
  UTF8ToString(ptr: number): string;
  stringToUTF8(str: string, ptr: number, maxBytes: number): void;
  lengthBytesUTF8(str: string): number;
}

export type ModuleFactory = (options: {
  print?: (s: string) => void;
  printErr?: (s: string) => void;
}) => Promise<CoolPropModule>;

export interface LoadOptions {
  /** Emscripten module factory. Defaults to the bundled `dist/coolprop.js`. */
  moduleFactory?: ModuleFactory;
  /** Module stdout. Default: ignored. */
  print?: (s: string) => void;
  /** Module stderr. Default: `console.error`. */
  printErr?: (s: string) => void;
}

/** Loads the wasm module. Load once and share the returned instance. */
export async function loadCoolProp(options: LoadOptions = {}): Promise<CoolProp> {
  const factory = options.moduleFactory ?? (await defaultFactory());
  const m = await factory({
    print: options.print ?? (() => {}),
    printErr: options.printErr ?? ((s) => console.error(s)),
  });

  /** Reads a string returned by the module and releases it. */
  function take(ptr: number): string {
    try {
      return m.UTF8ToString(ptr);
    } finally {
      m._coolprop_free_string(ptr);
    }
  }

  /** Calls `fn` with `request` as JSON in module memory, then frees both. */
  function call(fn: (request: number) => number, request: unknown): string {
    const json = JSON.stringify(request);
    const size = m.lengthBytesUTF8(json) + 1;
    const ptr = m._malloc(size);
    try {
      m.stringToUTF8(json, ptr, size);
      return take(fn(ptr));
    } finally {
      m._free(ptr);
    }
  }

  let catalog: readonly FluidData[] | undefined;
  let schema: Schema | undefined;
  const getSchema = () => (schema ??= unwrap<Schema>(take(m._coolprop_schema())));

  return {
    version: () => unwrap<string>(take(m._coolprop_version())),
    catalog: () => (catalog ??= Object.freeze(unwrap<FluidData[]>(take(m._coolprop_catalog())))),
    inputs: () => getSchema().inputs,
    pairs: () => getSchema().pairs,
    properties: () => getSchema().properties,
    phases: () => getSchema().phases,
    fluid(name: string): Fluid {
      const info = unwrap<{ data: FluidData; critical: CriticalPoint }>(
        call(m._coolprop_fluid, { fluid: name }),
      );
      const canonical = info.data.name;
      return {
        name: canonical,
        data: info.data,
        critical: info.critical,
        state: (inputs) => unwrap<State>(call(m._coolprop_state, { fluid: canonical, inputs })),
        states: (inputs) =>
          unwrap<Envelope<State>[]>(call(m._coolprop_states, { fluid: canonical, inputs })).map(
            (item) => ("ok" in item ? item.ok : new CoolPropError(item.error)),
          ),
      };
    },
  };
}

function unwrap<T>(json: string): T {
  const envelope = JSON.parse(json) as Envelope<T>;
  if ("error" in envelope) {
    throw new CoolPropError(envelope.error);
  }
  return envelope.ok;
}

async function defaultFactory(): Promise<ModuleFactory> {
  try {
    // @ts-expect-error: generated by `docker compose run --rm build-npm`, untyped.
    const mod = await import("../dist/coolprop.js");
    return mod.default as ModuleFactory;
  } catch (e) {
    throw new Error(
      "@luisbedoia/coolprop-rs-wasm: dist/coolprop.js is missing. " +
        "Build it with `docker compose run --rm build-npm` from the repository root. " +
        `Cause: ${(e as Error).message}`,
    );
  }
}
