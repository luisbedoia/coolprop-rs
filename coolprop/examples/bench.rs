use std::ffi::{CString, c_char, c_long};
use std::hint::black_box;
use std::time::Instant;

use coolprop::plot::{Diagram, DisplayUnit, PlotProperty, PropertyPlot};
use coolprop::{Fluid, Input, InputKind, Variant};

fn per(label: &str, n: usize, mut f: impl FnMut(usize)) -> f64 {
    for i in 0..n.min(50) { f(i); }
    let t = Instant::now();
    for i in 0..n { f(i); }
    let us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
    println!("{label:52} {us:9.2} µs");
    us
}

fn main() {
    // ── Raw C API ───────────────────────────────────────────────────────────
    let c = |s: &str| CString::new(s).unwrap();
    let mut err: c_long = 0;
    let mut msg = [0 as c_char; 1024];
    let h = unsafe { coolprop_sys::AbstractState_factory(c("HEOS").as_ptr(), c("Water").as_ptr(), &mut err, msg.as_mut_ptr(), 1024) };
    let pt = unsafe { coolprop_sys::get_input_pair_index(c("PT_INPUTS").as_ptr()) };
    let pq = unsafe { coolprop_sys::get_input_pair_index(c("PQ_INPUTS").as_ptr()) };
    let idx = |n: &str| unsafe { coolprop_sys::get_param_index(c(n).as_ptr()) };
    let basic: Vec<c_long> = ["P", "T", "D", "H", "S", "U", "Q", "Phase"].iter().map(|n| idx(n)).collect();
    let transport: Vec<c_long> = ["Cpmass", "Cvmass", "viscosity", "conductivity", "Prandtl", "Gmass", "Z", "speed_of_sound"].iter().map(|n| idx(n)).collect();
    let upd = |pair: c_long, a: f64, b: f64| unsafe {
        let mut e = 0; let mut m = [0 as c_char; 1024];
        coolprop_sys::AbstractState_update(h, pair, a, b, &mut e, m.as_mut_ptr(), 1024);
    };
    let out = |k: c_long| unsafe {
        let mut e = 0; let mut m = [0 as c_char; 1024];
        coolprop_sys::AbstractState_keyed_output(h, k, &mut e, m.as_mut_ptr(), 1024)
    };
    let n = 20_000;
    println!("== Water, raw C API");
    per("update PT liquid (T varies)", n, |i| upd(pt, 101325.0, 300.0 + (i % 50) as f64 * 0.5));
    per("update PT vapor (T varies)", n, |i| upd(pt, 101325.0, 400.0 + (i % 50) as f64 * 0.5));
    per("update PQ (x varies)", n, |i| upd(pq, 101325.0, (i % 50) as f64 / 50.0));
    upd(pt, 101325.0, 300.0);
    per("8 basic outputs (P T D H S U Q Phase)", n, |_| { for &k in &basic { black_box(out(k)); } });
    per("8 other outputs (cp cv μ λ Pr g Z a), cached state", n, |_| { for &k in &transport { black_box(out(k)); } });
    per("update PT + 8 other outputs (fresh caches)", n, |i| { upd(pt, 101325.0, 300.0 + (i % 50) as f64 * 0.5); for &k in &transport { black_box(out(k)); } });
    per("get_param_index by name (×16, CString)", n, |_| { for n in ["P","T","D","H","S","U","Q","Phase","Cpmass","Cvmass","viscosity","conductivity","Prandtl","Gmass","Z","speed_of_sound"] { black_box(idx(n)); } });
    per("unspecify_phase", n, |_| unsafe { let mut e = 0; let mut m = [0 as c_char; 1024]; coolprop_sys::AbstractState_unspecify_phase(h, &mut e, m.as_mut_ptr(), 1024); });

    // ── Library ────────────────────────────────────────────────────────────
    println!("== Library");
    let w = Fluid::new(Variant::Water).unwrap();
    per("Fluid::state PT liquid", n, |i| { black_box(w.state(Input::Pressure(101325.0), Input::Temperature(300.0 + (i % 50) as f64 * 0.5)).ok()); });
    per("Fluid::state PQ", n, |i| { black_box(w.state(Input::Pressure(101325.0), Input::Quality((i % 50) as f64 / 50.0)).ok()); });
    let ph = Diagram::new(PlotProperty::Enthalpy, PlotProperty::Pressure).unwrap();
    let c_unit = DisplayUnit { scale: 1.0, offset: -273.15 };
    for (name, variant) in [("Water", Variant::Water), ("Air", Variant::from_name("Air").unwrap())] {
        let f = Fluid::new(variant).unwrap();
        println!("-- {name}, p–h diagram");
        per("PropertyPlot::new (dome, 120 pts)", 30, |_| { black_box(PropertyPlot::with_resolution(&f, coolprop::plot::Limits::default_for(&f), 120).unwrap()); });
        let plot = PropertyPlot::new(&f).unwrap();
        per("range(x) + range(y)", 30, |_| { black_box(plot.range(PlotProperty::Enthalpy)); black_box(plot.range(PlotProperty::Pressure)); });
        per("suggested_values T (7)", 30, |_| { black_box(plot.suggested_values(InputKind::Temperature, 7, &ph, c_unit)); });
        let ts = plot.suggested_values(InputKind::Temperature, 7, &ph, c_unit);
        let us = per("7 isotherms × 120 pts", 10, |_| { black_box(plot.isolines(InputKind::Temperature, &ts, 120)); });
        println!("   ≈ {:.1} µs per isotherm point", us / 840.0);
        let ds = plot.suggested_values(InputKind::Density, 7, &ph, DisplayUnit::SI);
        per("7 isochores × 120 pts", 10, |_| { black_box(plot.isolines(InputKind::Density, &ds, 120)); });
    }
}
