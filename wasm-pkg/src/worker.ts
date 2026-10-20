/**
 * Web Worker entry used by `loadCoolPropWorker`: loads the wasm module off
 * the main thread and answers requests with the synchronous API.
 */

import { CoolPropError, loadCoolProp, type CoolProp, type Fluid } from "./index.js";
import type { ReferenceState } from "./types.js";
import type { FluidResult, InitResult, Request, Response, StatesResult, WireError } from "./protocol.js";

// Typed locally: the DOM lib types `self` as a Window.
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<Request>) => void) | null;
  postMessage(message: Response): void;
};

/** Loaded on `init`, the client's first request, in the locale it asks for. */
let ready: Promise<CoolProp> | undefined;
/** Open fluids by name and reference. */
const fluids = new Map<string, Fluid>();

function toWire(e: unknown): WireError {
  return e instanceof CoolPropError
    ? { kind: e.kind, message: e.message }
    : { message: e instanceof Error ? e.message : String(e) };
}

async function handle(req: Request): Promise<unknown> {
  if (req.method === "init") ready ??= loadCoolProp({ locale: req.locale });
  const cp = await (ready ??= loadCoolProp());
  const open = (name: string, reference: ReferenceState) => {
    const key = `${name}|${reference}`;
    let fluid = fluids.get(key);
    if (!fluid) {
      fluid = cp.fluid(name, { reference });
      fluids.set(key, fluid);
    }
    return fluid;
  };
  switch (req.method) {
    case "init":
      return {
        version: cp.version(),
        catalog: [...cp.catalog()],
        inputs: [...cp.inputs()],
        pairs: cp.pairs().map(([a, b]) => [a, b]),
        properties: [...cp.properties()],
        phases: [...cp.phases()],
        plot_properties: [...cp.plotProperties()],
        diagrams: [...cp.diagrams()],
      } satisfies InitResult;
    case "fluid": {
      const f = open(req.name, req.reference);
      return {
        name: f.name,
        data: f.data,
        critical: f.critical,
        reference: f.reference,
      } satisfies FluidResult;
    }
    case "state":
      return open(req.fluid, req.reference).state(req.inputs);
    case "diagram":
      return open(req.fluid, req.reference).diagram(req.request);
    case "states":
      return open(req.fluid, req.reference)
        .states(req.inputs)
        .map((r) =>
          r instanceof CoolPropError ? { error: { kind: r.kind, message: r.message } } : { ok: r },
        ) satisfies StatesResult;
  }
}

// Requests are handled in arrival order: each waits on the same `ready`.
scope.onmessage = async (e) => {
  const { id } = e.data;
  try {
    scope.postMessage({ id, ok: await handle(e.data) });
  } catch (err) {
    scope.postMessage({ id, error: toWire(err) });
  }
};
