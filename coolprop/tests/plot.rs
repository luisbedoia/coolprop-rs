//! `plot`: the diagram catalog, the saturation dome and isolines. With no
//! external reference for most diagrams, these check physics: each isoline
//! keeps its property, crosses the dome the right way and stays monotonic.

use coolprop::plot::{Diagram, DisplayUnit, Limits, PlotProperty, PropertyPlot, Scale};
use coolprop::{Fluid, Input, InputKind, PropsError, State, Variant};

fn water() -> Fluid {
    Fluid::new(Variant::Water).unwrap()
}

fn solved(states: &[Option<State>]) -> Vec<State> {
    states.iter().flatten().copied().collect()
}

#[track_caller]
fn assert_rel(actual: f64, expected: f64, tol: f64, label: &str) {
    let rel = (actual - expected).abs() / expected.abs().max(1e-12);
    assert!(
        rel <= tol,
        "{label}: {actual} vs {expected} (rel {rel:.2e})"
    );
}

// ── Catalog ─────────────────────────────────────────────────────────────────

#[test]
fn catalog_has_the_engineering_diagrams_in_order() {
    let ids: Vec<String> = Diagram::all().iter().map(Diagram::id).collect();
    assert_eq!(
        ids,
        [
            "pressure_enthalpy",
            "temperature_entropy",
            "enthalpy_entropy",
            "pressure_specific_volume",
            "temperature_specific_volume",
            "pressure_temperature",
        ]
    );
}

#[test]
fn phase_diagram_has_a_linear_pressure_axis() {
    let pt = Diagram::from_id("pressure_temperature").unwrap();
    assert_eq!((pt.x.scale, pt.y.scale), (Scale::Linear, Scale::Linear));
    let ph = Diagram::from_id("pressure_enthalpy").unwrap();
    assert_eq!(ph.y.scale, Scale::Log);
}

#[test]
fn default_scales_are_log_for_wide_ranging_properties() {
    let ph = Diagram::new(PlotProperty::Enthalpy, PlotProperty::Pressure).unwrap();
    assert_eq!((ph.x.scale, ph.y.scale), (Scale::Linear, Scale::Log));
    let pv = Diagram::new(PlotProperty::SpecificVolume, PlotProperty::Pressure).unwrap();
    assert_eq!((pv.x.scale, pv.y.scale), (Scale::Log, Scale::Log));
}

#[test]
fn same_quantity_twice_is_not_a_diagram() {
    let r = Diagram::new(PlotProperty::Density, PlotProperty::SpecificVolume);
    assert!(matches!(r, Err(PropsError::InvalidInput(_))));
}

#[test]
fn isoline_kinds_exclude_axes_and_degenerate_quality() {
    let ph = Diagram::new(PlotProperty::Enthalpy, PlotProperty::Pressure).unwrap();
    let kinds = ph.isoline_kinds();
    assert!(!kinds.contains(&InputKind::Pressure) && !kinds.contains(&InputKind::Enthalpy));
    assert!(kinds.contains(&InputKind::Temperature) && kinds.contains(&InputKind::Quality));

    let pt = Diagram::new(PlotProperty::Temperature, PlotProperty::Pressure).unwrap();
    assert!(!pt.isoline_kinds().contains(&InputKind::Quality));

    // Specific volume on an axis excludes isochores too.
    let pv = Diagram::new(PlotProperty::SpecificVolume, PlotProperty::Pressure).unwrap();
    assert!(!pv.isoline_kinds().contains(&InputKind::Density));
}

// ── Saturation dome ─────────────────────────────────────────────────────────

#[test]
fn dome_branches_rise_and_close_at_the_critical_point() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let dome = plot.dome();
    assert_eq!(dome.liquid.len(), dome.vapor.len());

    for branch in [&dome.liquid, &dome.vapor] {
        for pair in branch.windows(2) {
            assert!(pair[1].temperature() > pair[0].temperature());
            assert!(pair[1].pressure() > pair[0].pressure());
        }
    }
    let n = dome.liquid.len() - 1;
    for i in 0..n {
        assert!(dome.liquid[i].density() > dome.vapor[i].density());
        assert_eq!(dome.liquid[i].quality(), Some(0.0));
        assert_eq!(dome.vapor[i].quality(), Some(1.0));
    }
    let crit = w.critical();
    assert_eq!(dome.liquid[n], dome.vapor[n]);
    assert_rel(
        dome.liquid[n].temperature(),
        crit.temperature,
        1e-9,
        "apex T",
    );
    assert_rel(dome.liquid[n].pressure(), crit.pressure, 1e-6, "apex p");
}

// ── Isolines ────────────────────────────────────────────────────────────────

#[test]
fn every_isoline_keeps_its_property() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let cases = [
        (InputKind::Pressure, 1e5),
        (InputKind::Temperature, 400.0),
        (InputKind::Density, 50.0),
        (InputKind::Enthalpy, 2.0e6),
        (InputKind::Entropy, 6000.0),
        (InputKind::InternalEnergy, 1.5e6),
        (InputKind::Quality, 0.3),
    ];
    for (kind, value) in cases {
        let iso = plot.isoline(kind, value, 60);
        let states = solved(&iso.states);
        assert!(
            states.len() >= 30,
            "{kind:?}: only {} points solved",
            states.len()
        );
        for s in states {
            let held = match kind {
                InputKind::Pressure => s.pressure(),
                InputKind::Temperature => s.temperature(),
                InputKind::Density => s.density(),
                InputKind::Enthalpy => s.enthalpy(),
                InputKind::Entropy => s.entropy(),
                InputKind::InternalEnergy => s.internal_energy(),
                InputKind::Quality => s.quality().unwrap(),
            };
            assert_rel(held, value, 1e-6, &format!("{kind:?}"));
        }
    }
}

#[test]
fn subcritical_isobar_crosses_the_dome_flat_with_rising_entropy() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let states = solved(&plot.isoline(InputKind::Pressure, 1e5, 90).states);

    let in_dome: Vec<&State> = states.iter().filter(|s| s.quality().is_some()).collect();
    assert!(
        in_dome.len() >= 20,
        "the dome segment is sampled by quality"
    );
    let t_sat = in_dome[0].temperature();
    assert!(
        in_dome
            .iter()
            .all(|s| (s.temperature() - t_sat).abs() < 1e-6)
    );

    for pair in states.windows(2) {
        assert!(
            pair[1].entropy() > pair[0].entropy(),
            "s rises along the isobar"
        );
    }
}

#[test]
fn subcritical_isotherm_crosses_the_dome_flat_with_falling_enthalpy() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let states = solved(&plot.isoline(InputKind::Temperature, 400.0, 90).states);

    let in_dome: Vec<&State> = states.iter().filter(|s| s.quality().is_some()).collect();
    assert!(in_dome.len() >= 20);
    let p_sat = in_dome[0].pressure();
    assert!(
        in_dome
            .iter()
            .all(|s| (s.pressure() / p_sat - 1.0).abs() < 1e-9)
    );

    // Vapor → dome → compressed liquid: p never falls; h falls through the
    // vapor and the dome but rises again in the liquid, where
    // (∂h/∂p)_T = v(1 − βT) > 0.
    let liquid_from = states
        .iter()
        .position(|s| s.quality().is_none() && s.pressure() > p_sat)
        .unwrap();
    for pair in states.windows(2) {
        assert!(pair[1].pressure() >= pair[0].pressure(), "p never falls");
    }
    for pair in states[..liquid_from].windows(2) {
        assert!(
            pair[1].enthalpy() < pair[0].enthalpy(),
            "h falls before the liquid"
        );
    }
    for pair in states[liquid_from..].windows(2) {
        assert!(
            pair[1].enthalpy() > pair[0].enthalpy(),
            "h rises in the liquid"
        );
    }
}

#[test]
fn supercritical_isobar_and_isotherm_never_enter_the_dome() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let crit = w.critical();
    for (kind, value) in [
        (InputKind::Pressure, crit.pressure * 1.5),
        (InputKind::Temperature, crit.temperature * 1.2),
    ] {
        let iso = plot.isoline(kind, value, 50);
        assert_eq!(iso.states.len(), 50);
        assert!(iso.states.iter().flatten().all(|s| s.quality().is_none()));
    }
}

#[test]
fn quality_zero_isoline_is_the_liquid_branch() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let q0 = solved(&plot.isoline(InputKind::Quality, 0.0, 40).states);
    // Swept by pressure over the dome's span below the critical point.
    let liquid = &plot.dome().liquid;
    let (p_lo, p_hi) = (liquid[0].pressure(), liquid[liquid.len() - 2].pressure());
    assert_rel(q0[0].pressure(), p_lo, 1e-6, "starts with the dome");
    assert_rel(
        q0[q0.len() - 1].pressure(),
        p_hi,
        1e-6,
        "ends with the dome",
    );
    // Each point is the saturated liquid at its own temperature.
    for s in &q0 {
        let liq = w
            .state(Input::Quality(0.0), Input::Temperature(s.temperature()))
            .unwrap();
        assert_rel(s.enthalpy(), liq.enthalpy(), 1e-6, "on the liquid branch");
    }
    assert!(q0.iter().all(|s| s.quality() == Some(0.0)));
}

#[test]
fn out_of_range_quality_gives_an_empty_isoline() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    assert!(plot.isoline(InputKind::Quality, 1.5, 20).states.is_empty());
}

#[test]
fn projection_maps_unsolved_points_to_nan() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    // An impossible value (water is not a fluid at 200 K) has no curve.
    assert!(
        plot.isoline(InputKind::Temperature, 200.0, 40)
            .states
            .is_empty()
    );
    // This isentrope leaves the domain partway: it has both kinds of point.
    let iso = plot.isoline(InputKind::Entropy, 12_668.0, 40);
    assert!(iso.states.iter().any(Option::is_none));
    assert!(iso.states.iter().any(Option::is_some));
    let (x, y) = iso.project(PlotProperty::Enthalpy, PlotProperty::Pressure);
    assert_eq!(x.len(), iso.states.len());
    for (i, s) in iso.states.iter().enumerate() {
        assert_eq!(s.is_none(), x[i].is_nan());
        assert_eq!(s.is_none(), y[i].is_nan());
    }
    // Specific volume is the inverse of density.
    let (v, rho) = plot
        .isoline(InputKind::Pressure, 1e5, 20)
        .project(PlotProperty::SpecificVolume, PlotProperty::Density);
    for (v, rho) in v.iter().zip(&rho).filter(|(v, _)| v.is_finite()) {
        assert_rel(v * rho, 1.0, 1e-12, "v·ρ");
    }
}

/// Every family on every diagram yields curves with data: the whole catalog
/// is computable.
#[test]
fn every_diagram_and_family_is_computable() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    for d in Diagram::all() {
        assert!(plot.range(d.x.property).is_some(), "{d:?} x range");
        assert!(plot.range(d.y.property).is_some(), "{d:?} y range");
        for kind in d.isoline_kinds() {
            let values = plot.suggested_values(kind, 4, &d, DisplayUnit::SI);
            assert_eq!(values.len(), 4, "{kind:?}");
            for iso in plot.isolines(kind, &values, 40) {
                let (x, y) = iso.project(d.x.property, d.y.property);
                let finite = x
                    .iter()
                    .zip(&y)
                    .filter(|(a, b)| a.is_finite() && b.is_finite());
                assert!(finite.count() >= 10, "{d:?} {kind:?}={}", iso.value);
            }
        }
    }
}

// ── Ranges, values and limits ───────────────────────────────────────────────

#[test]
fn axis_ranges_follow_the_limits() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let l = plot.limits();
    let (p_lo, p_hi) = plot.range(PlotProperty::Pressure).unwrap();
    // Solving the corner states round-trips p to ~1e-8.
    assert_rel(p_lo, l.p_min, 1e-6, "p min");
    assert_rel(p_hi, l.p_max, 1e-6, "p max");
    let (t_lo, t_hi) = plot.range(PlotProperty::Temperature).unwrap();
    assert!(t_lo <= l.t_min * 1.001 && t_hi >= l.t_max * 0.999);
}

fn ph() -> Diagram {
    Diagram::new(PlotProperty::Enthalpy, PlotProperty::Pressure).unwrap()
}

const CELSIUS: DisplayUnit = DisplayUnit {
    scale: 1.0,
    offset: -273.15,
};

const KPA: DisplayUnit = DisplayUnit {
    scale: 1e-3,
    offset: 0.0,
};

/// Whether `v` is 1, 2, 2.5 or 5 × 10ᵏ times an integer for some k no finer
/// than a hundredth of `v`: a value people would pick.
fn is_round(v: f64) -> bool {
    let k = v.abs().log10().floor() - 2.0;
    [1.0, 2.0, 2.5, 5.0].iter().any(|m| {
        (0..=3).any(|e| {
            let step = m * 10f64.powf(k + e as f64);
            ((v / step).round() * step - v).abs() <= 1e-9 * v.abs().max(1.0)
                && (v / step).round().abs() <= 100.0
        })
    })
}

#[test]
fn suggested_quality_spans_zero_to_one() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    assert_eq!(
        plot.suggested_values(InputKind::Quality, 3, &ph(), CELSIUS),
        [0.25, 0.5, 0.75]
    );
}

/// Off the axes they would lie flat on, families are an even grid of round
/// values in the display unit, inside the dome's span.
#[test]
fn suggested_values_are_a_round_grid_in_display_units() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let hs = Diagram::new(PlotProperty::Entropy, PlotProperty::Enthalpy).unwrap();

    let t = plot.suggested_values(InputKind::Temperature, 7, &hs, CELSIUS);
    let shown: Vec<f64> = t.iter().map(|&v| CELSIUS.show(v)).collect();
    assert_eq!(shown.len(), 7);
    let step = shown[1] - shown[0];
    for pair in shown.windows(2) {
        assert_rel(pair[1] - pair[0], step, 1e-9, "uniform step");
    }
    assert!(shown.iter().all(|&v| is_round(v)), "{shown:?}");
    let (lo, hi) = plot.dome_range(PlotProperty::Temperature).unwrap();
    assert!(t[0] > lo && t[6] < hi);

    // Log quantities step 1-2-5 per decade (or coarser/finer sequences).
    let p = plot.suggested_values(InputKind::Pressure, 6, &hs, KPA);
    let shown: Vec<f64> = p.iter().map(|&v| KPA.show(v)).collect();
    assert_eq!(shown.len(), 6);
    assert!(shown.iter().all(|&v| is_round(v)), "{shown:?}");
    assert!(shown.windows(2).all(|w| w[1] > w[0]));
}

/// Isotherms lie flat inside the dome of P–h at the saturation pressure,
/// which grows about exponentially with temperature: spread evenly in T,
/// they would bunch up at the top of the log-pressure axis. They are spread
/// by saturation pressure instead, and still read as round temperatures.
#[test]
fn isotherms_on_a_pressure_axis_are_evenly_spaced_on_it() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let t = plot.suggested_values(InputKind::Temperature, 7, &ph(), CELSIUS);
    assert_eq!(t.len(), 7);
    let shown: Vec<f64> = t.iter().map(|&v| CELSIUS.show(v)).collect();
    assert!(shown.iter().all(|&v| is_round(v)), "{shown:?}");

    let ln_p: Vec<f64> = t
        .iter()
        .map(|&t| {
            w.state(Input::Temperature(t), Input::Quality(0.0))
                .unwrap()
                .pressure()
                .ln()
        })
        .collect();
    let gaps: Vec<f64> = ln_p.windows(2).map(|w| w[1] - w[0]).collect();
    let (min, max) = gaps
        .iter()
        .fold((f64::INFINITY, 0f64), |(lo, hi), &g| (lo.min(g), hi.max(g)));
    assert!(max / min < 1.8, "log-pressure gaps {gaps:?}");

    // Isobars on a temperature axis, likewise.
    let ts = Diagram::new(PlotProperty::Entropy, PlotProperty::Temperature).unwrap();
    let p = plot.suggested_values(InputKind::Pressure, 7, &ts, KPA);
    let shown: Vec<f64> = p.iter().map(|&v| KPA.show(v)).collect();
    assert!(shown.iter().all(|&v| is_round(v)), "{shown:?}");
    let t_sat: Vec<f64> = p
        .iter()
        .map(|&p| {
            w.state(Input::Pressure(p), Input::Quality(0.0))
                .unwrap()
                .temperature()
        })
        .collect();
    let gaps: Vec<f64> = t_sat.windows(2).map(|w| w[1] - w[0]).collect();
    let (min, max) = gaps
        .iter()
        .fold((f64::INFINITY, 0f64), |(lo, hi), &g| (lo.min(g), hi.max(g)));
    assert!(max / min < 1.8, "saturation temperature gaps {gaps:?}");
}

/// On P–T, pressure is linear: isochores spread by the pressure where they
/// leave the saturation curve, not crowded at its foot as a log grid of
/// densities would be.
#[test]
fn isochores_on_a_linear_pressure_axis_are_evenly_spaced_on_it() {
    let w = water();
    let plot = PropertyPlot::new(&w).unwrap();
    let pt = Diagram::from_id("pressure_temperature").unwrap();
    let rho = plot.suggested_values(InputKind::Density, 7, &pt, DisplayUnit::SI);
    assert_eq!(rho.len(), 7);
    assert!(rho.iter().all(|&v| is_round(v)), "{rho:?}");
    let p_dew: Vec<f64> = rho
        .iter()
        .map(|&d| {
            let t = w.state(Input::Density(d), Input::Quality(1.0)).unwrap();
            t.pressure()
        })
        .collect();
    let gaps: Vec<f64> = p_dew.windows(2).map(|w| w[1] - w[0]).collect();
    let (min, max) = gaps
        .iter()
        .fold((f64::INFINITY, 0f64), |(lo, hi), &g| (lo.min(g), hi.max(g)));
    assert!(max / min < 1.8, "dew pressure gaps {gaps:?}");
}

#[test]
fn invalid_limits_are_rejected() {
    let w = water();
    let mut l = Limits::default_for(&w);
    l.t_max = l.t_min;
    assert!(matches!(
        PropertyPlot::with_limits(&w, l),
        Err(PropsError::InvalidInput(_))
    ));
}

/// Pseudo-pure blends glide across the dome: bubble and dew temperatures
/// differ, and the isobar must leave the dome at the dew point.
#[test]
fn blend_isobar_glides_across_the_dome() {
    let Some(variant) = Variant::from_name("R410A") else {
        return; // not in this build's COOLPROP_FLUIDS
    };
    let fluid = Fluid::new(variant).unwrap();
    let plot = PropertyPlot::new(&fluid).unwrap();
    let states = solved(&plot.isoline(InputKind::Pressure, 1e6, 90).states);
    let in_dome: Vec<&State> = states.iter().filter(|s| s.quality().is_some()).collect();
    let (first, last) = (in_dome[0], in_dome[in_dome.len() - 1]);
    assert!(
        last.temperature() > first.temperature(),
        "temperature glide"
    );
    for pair in states.windows(2) {
        assert!(pair[1].entropy() > pair[0].entropy());
    }
}

/// Indices of the isoline's in-dome states, if that stretch is complete:
/// enough points, and none unsolved between its first and last.
fn complete_dome_stretch(states: &[Option<State>]) -> Result<(usize, usize), String> {
    let in_dome: Vec<usize> = states
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_some_and(|s| s.quality().is_some()))
        .map(|(i, _)| i)
        .collect();
    if in_dome.len() < 20 {
        return Err(format!("only {} points in the dome", in_dome.len()));
    }
    let (first, last) = (in_dome[0], in_dome[in_dome.len() - 1]);
    match states[first..=last].iter().position(Option::is_none) {
        Some(gap) => Err(format!("unsolved point at {} inside the dome", first + gap)),
        None => Ok((first, last)),
    }
}

/// Subcritical isobars and isotherms cross the dome without gaps for every
/// fluid in the build — pseudo-pure blends included, whose two-phase states
/// cannot all be solved the way pure fluids' can.
#[test]
fn every_fluid_crosses_the_dome_without_gaps() {
    for &variant in Variant::ALL {
        let fluid = Fluid::new(variant).unwrap();
        let plot = PropertyPlot::new(&fluid).unwrap();
        for kind in [InputKind::Pressure, InputKind::Temperature] {
            for value in plot.suggested_values(kind, 3, &ph(), DisplayUnit::SI) {
                let iso = plot.isoline(kind, value, 90);
                if let Err(e) = complete_dome_stretch(&iso.states) {
                    panic!("{} {kind:?}={value:.4e}: {e}", variant.name());
                }
            }
        }
    }
}

/// Inside the dome a blend's isotherm is not flat: pressure glides from the
/// dew point up to the bubble point.
#[test]
fn blend_isotherm_glides_across_the_dome() {
    let Some(variant) = Variant::from_name("R410A") else {
        return; // not in this build's COOLPROP_FLUIDS
    };
    let fluid = Fluid::new(variant).unwrap();
    let plot = PropertyPlot::new(&fluid).unwrap();
    let t = plot.suggested_values(InputKind::Temperature, 3, &ph(), DisplayUnit::SI)[1];
    let iso = plot.isoline(InputKind::Temperature, t, 90);
    let (first, last) = complete_dome_stretch(&iso.states).unwrap();
    let dome: Vec<State> = solved(&iso.states[first..=last]);
    for pair in dome.windows(2) {
        assert!(
            pair[1].pressure() > pair[0].pressure(),
            "pressure glides up"
        );
        assert_rel(pair[1].temperature(), t, 1e-9, "temperature stays fixed");
    }
    // Continuous with the single-phase stretches: no steps at either end.
    let dew = fluid
        .state(Input::Temperature(t), Input::Quality(1.0))
        .unwrap();
    let bubble = fluid
        .state(Input::Temperature(t), Input::Quality(0.0))
        .unwrap();
    assert_rel(
        dome[0].pressure(),
        dew.pressure(),
        1e-6,
        "starts at the dew point",
    );
    assert_rel(
        dome[dome.len() - 1].pressure(),
        bubble.pressure(),
        1e-6,
        "ends at the bubble point",
    );
}
