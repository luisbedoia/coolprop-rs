/**
 * Asynchronous API: the same operations as `loadCoolProp`, solved in a Web
 * Worker so the main thread never blocks.
 */

import { CoolPropError, type CoolPropInfo } from "./index.js";
import type {
  FluidResult,
  InitResult,
  Request,
  RequestBody,
  Response,
  StatesResult,
  WireError,
} from "./protocol.js";
import type { CriticalPoint, FluidData, State, StateInputs } from "./types.js";

/** A curated fluid whose states are solved in the worker. */
export interface WorkerFluid {
  /** Canonical CoolProp name. */
  readonly name: string;
  readonly data: FluidData;
  /** Critical point of the equation of state. */
  readonly critical: CriticalPoint;
  /** Solves the state fixed by two inputs. Rejects with `CoolPropError`. */
  state(inputs: StateInputs): Promise<State>;
  /** Solves many states; each entry is a `State` or that point's `CoolPropError`. */
  states(inputs: readonly StateInputs[]): Promise<(State | CoolPropError)[]>;
}

/**
 * Catalogs and schema are fetched once at load and answered synchronously;
 * only solving goes through the worker.
 */
export interface CoolPropWorker extends CoolPropInfo {
  /** Opens a fluid by canonical name or alias. Rejects with `CoolPropError`. */
  fluid(name: string): Promise<WorkerFluid>;
  /** Stops the worker; pending and later calls reject. */
  terminate(): void;
}

export interface WorkerOptions {
  /**
   * A worker running this package's worker script, for setups where the
   * default `new URL("./worker.js", import.meta.url)` cannot be resolved,
   * e.g. `new Worker(new URL("@luisbedoia/coolprop-rs-wasm/worker",
   * import.meta.url), { type: "module" })`.
   */
  worker?: Worker;
}

/** Starts the worker and loads the wasm module in it. */
export async function loadCoolPropWorker(options: WorkerOptions = {}): Promise<CoolPropWorker> {
  const worker =
    options.worker ??
    new Worker(new URL("./worker.js", import.meta.url), { type: "module", name: "coolprop" });

  const pending = new Map<number, { resolve(v: unknown): void; reject(e: Error): void }>();
  let nextId = 0;
  let failure: Error | undefined;

  const fail = (error: Error) => {
    failure ??= error;
    for (const p of pending.values()) p.reject(failure);
    pending.clear();
  };

  worker.onmessage = (e: MessageEvent<Response>) => {
    const p = pending.get(e.data.id);
    if (!p) return;
    pending.delete(e.data.id);
    if ("error" in e.data) p.reject(fromWire(e.data.error));
    else p.resolve(e.data.ok);
  };
  worker.onerror = (e) => {
    e.preventDefault();
    fail(new Error(`coolprop worker failed: ${e.message || "could not load the worker script"}`));
  };
  worker.onmessageerror = () => fail(new Error("coolprop worker sent an unreadable message"));

  function request<T>(req: RequestBody): Promise<T> {
    if (failure) return Promise.reject(failure);
    const id = nextId++;
    return new Promise<T>((resolve, reject) => {
      pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      worker.postMessage({ ...req, id } as Request);
    });
  }

  let init: InitResult;
  try {
    init = await request<InitResult>({ method: "init" });
  } catch (e) {
    worker.terminate();
    throw e;
  }
  const catalog = Object.freeze(init.catalog);

  return {
    version: () => init.version,
    catalog: () => catalog,
    inputs: () => init.inputs,
    pairs: () => init.pairs,
    properties: () => init.properties,
    phases: () => init.phases,
    async fluid(name: string): Promise<WorkerFluid> {
      const f = await request<FluidResult>({ method: "fluid", name });
      return {
        name: f.name,
        data: f.data,
        critical: f.critical,
        state: (inputs) => request<State>({ method: "state", fluid: f.name, inputs }),
        states: async (inputs) =>
          (await request<StatesResult>({ method: "states", fluid: f.name, inputs })).map((item) =>
            "ok" in item ? item.ok : new CoolPropError(item.error),
          ),
      };
    },
    terminate() {
      worker.terminate();
      fail(new Error("coolprop worker was terminated"));
    },
  };
}

function fromWire(e: WireError): CoolPropError | Error {
  return e.kind ? new CoolPropError({ kind: e.kind, message: e.message }) : new Error(e.message);
}
