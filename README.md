# coolprop-rs

Rust bindings to [CoolProp](https://coolprop.org) and a WebAssembly build of
them for the browser. CoolProp does the thermodynamics; this project makes it
easier to use from Rust and JavaScript.

```rust
use coolprop::{Fluid, Input, Variant};

let water = Fluid::new(Variant::Water)?;
let s = water.state(Input::Pressure(101_325.0), Input::Temperature(298.15))?;
s.enthalpy(); // J/kg
```

Everything is SI. Besides states, the library describes its own API (inputs,
valid pairs, properties, phases) and computes data for property diagrams:
saturation dome and isolines for p–h, T–s, h–s, p–v, T–v and p–T.

## Crates

| Path | What it is |
| --- | --- |
| [`coolprop-sys`](coolprop-sys) | Builds CoolProp (v8, git submodule) and its raw C bindings |
| [`coolprop`](coolprop) | Safe API: fluids, states, catalogs, diagrams |
| [`coolprop-wasm`](coolprop-wasm) | JSON bridge compiled to WebAssembly with Emscripten |
| [`wasm-pkg`](wasm-pkg) | npm package [`@luisbedoia/coolprop-rs-wasm`](wasm-pkg/README.md) |

Only a curated set of fluids is compiled in, chosen at build time with
`COOLPROP_FLUIDS`. The npm package ships 30 (water, air, CO₂, ammonia,
common refrigerants and hydrocarbons…); a plain native build defaults to a
smaller set.

## Building and testing

The CoolProp sources are a submodule:

```sh
git submodule update --init --recursive
```

Native builds need CMake, a C++ compiler and Python 3. The wasm toolchain
(Emscripten) lives in the Docker image:

```sh
docker compose run --rm test-native   # Rust tests
docker compose run --rm test-wasm     # same tests on wasm32-unknown-emscripten
docker compose run --rm build-npm     # npm package artifacts into wasm-pkg/dist/
docker compose run --rm test-npm      # TypeScript checks and browser tests
```

Releases are published to GitHub Packages from a `v<version>` tag; the
version lives once, in the root `Cargo.toml`.

## Credits

All property values come from CoolProp. If you use them in academic work,
cite it:

> Bell, I. H.; Wronski, J.; Quoilin, S.; Lemort, V. Pure and Pseudo-pure
> Fluid Thermophysical Property Evaluation and the Open-Source Thermophysical
> Property Library CoolProp. *Ind. Eng. Chem. Res.* **2014**, 53 (6),
> 2498–2508. [doi:10.1021/ie4033999](https://doi.org/10.1021/ie4033999)

Used by [Thermoprops](https://luisbedoia.github.io/thermoprops/).

## License

MIT, like CoolProp. See [LICENSE](LICENSE).
