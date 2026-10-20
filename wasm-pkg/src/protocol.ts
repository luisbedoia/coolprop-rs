/** Messages between `loadCoolPropWorker` and the worker script. Internal. */

import type {
  CriticalPoint,
  DiagramInfo,
  DiagramRequest,
  ErrorBody,
  ErrorKind,
  FluidData,
  InputInfo,
  InputName,
  Locale,
  PhaseInfo,
  PlotPropertyInfo,
  PropertyInfo,
  ReferenceState,
  State,
  StateInputs,
} from "./types.js";

/** A request before the client assigns its id. */
export type RequestBody = Request extends infer R ? (R extends unknown ? Omit<R, "id"> : never) : never;

export type Request =
  | { id: number; method: "init"; locale: Locale }
  | { id: number; method: "fluid"; name: string; reference: ReferenceState }
  | { id: number; method: "state"; fluid: string; reference: ReferenceState; inputs: StateInputs }
  | {
      id: number;
      method: "states";
      fluid: string;
      reference: ReferenceState;
      inputs: readonly StateInputs[];
    }
  | {
      id: number;
      method: "diagram";
      fluid: string;
      reference: ReferenceState;
      request: DiagramRequest;
    };

/** `kind` is absent for failures outside CoolProp (e.g. the module not loading). */
export interface WireError {
  kind?: ErrorKind;
  message: string;
}

export type Response = { id: number; ok: unknown } | { id: number; error: WireError };

export interface InitResult {
  version: string;
  catalog: FluidData[];
  inputs: InputInfo[];
  pairs: [InputName, InputName][];
  properties: PropertyInfo[];
  phases: PhaseInfo[];
  plot_properties: PlotPropertyInfo[];
  diagrams: DiagramInfo[];
}

export interface FluidResult {
  name: string;
  data: FluidData;
  critical: CriticalPoint;
  reference: ReferenceState;
}

/** Per-point failures always come from CoolProp, so they carry a `kind`. */
export type StatesResult = ({ ok: State } | { error: ErrorBody })[];
