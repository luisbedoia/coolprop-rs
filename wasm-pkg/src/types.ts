/** JSON shapes exchanged with the coolprop wasm module. All values are SI. */

/** Curated-catalog constants for one fluid. */
export interface FluidData {
  /** Canonical CoolProp name. */
  name: string;
  /** CAS registry number. */
  cas: string;
  /** Formula in CoolProp's LaTeX-ish notation; empty for pseudo-pure fluids. */
  formula: string;
  /** Other accepted names, e.g. "H2O", "R718". */
  aliases: string[];
  /** kg/mol */
  molar_mass: number;
  /** Acentric factor, dimensionless. */
  acentric: number;
  /** K — triple point, also the lowest valid temperature. */
  t_triple: number;
  /** Pa */
  p_triple: number;
  /** K — EOS upper temperature limit. */
  t_max: number;
  /** Pa — EOS upper pressure limit. */
  p_max: number;
}

/** Critical point of the fluid's equation of state. */
export interface CriticalPoint {
  /** K */
  temperature: number;
  /** Pa */
  pressure: number;
  /** kg/m³ */
  density: number;
}

/** Phase of a solved state. */
export type Phase =
  | "liquid"
  | "supercritical"
  | "supercritical_gas"
  | "supercritical_liquid"
  | "critical_point"
  | "gas"
  | "two_phase";

/** A fully solved state. `null` marks a property undefined for the state. */
export interface State {
  /** Pa */
  pressure: number;
  /** K */
  temperature: number;
  /** kg/m³ */
  density: number;
  /** J/kg */
  enthalpy: number;
  /** J/(kg·K) */
  entropy: number;
  /** J/kg */
  internal_energy: number;
  /** Vapor mass fraction in [0, 1]; null outside the two-phase region. */
  quality: number | null;
  phase: Phase | null;
  /** J/(kg·K) */
  cp: number | null;
  /** J/(kg·K) */
  cv: number | null;
  /** Pa·s */
  viscosity: number | null;
  /** W/(m·K) */
  conductivity: number | null;
  /** Dimensionless. */
  prandtl: number | null;
  /** J/kg */
  gibbs: number | null;
  /** Z = pv/RT, dimensionless. */
  compressibility: number | null;
  /** m/s */
  speed_of_sound: number | null;
}

/** Properties that can fix a state. */
export type InputName =
  | "pressure"
  | "temperature"
  | "density"
  | "enthalpy"
  | "entropy"
  | "internal_energy"
  | "quality";

/** Exactly the inputs `A` and `B`; any other input is a type error. */
type Pair<A extends InputName, B extends InputName> = { [K in A | B]: number } & {
  [K in Exclude<InputName, A | B>]?: never;
};

/** The two inputs that fix a state; only CoolProp's supported pairs type-check. */
export type StateInputs =
  | Pair<"pressure", "temperature">
  | Pair<"pressure", "quality">
  | Pair<"temperature", "quality">
  | Pair<"pressure", "density">
  | Pair<"pressure", "enthalpy">
  | Pair<"pressure", "entropy">
  | Pair<"pressure", "internal_energy">
  | Pair<"temperature", "density">
  | Pair<"temperature", "entropy">
  | Pair<"density", "quality">
  | Pair<"density", "enthalpy">
  | Pair<"density", "entropy">
  | Pair<"density", "internal_energy">
  | Pair<"enthalpy", "entropy">;

/** Why a call failed. */
export type ErrorKind =
  /** The name matches no curated fluid nor alias. */
  | "unknown_fluid"
  /** Malformed request, e.g. an unsupported input pair. */
  | "invalid_input"
  /** CoolProp could not solve it, e.g. a state outside the EOS range. */
  | "coolprop";

export interface ErrorBody {
  kind: ErrorKind;
  message: string;
}

/** Response envelope of every module call. */
export type Envelope<T> = { ok: T } | { error: ErrorBody };
