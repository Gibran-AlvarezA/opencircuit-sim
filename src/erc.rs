//! Reglas eléctricas (ERC): convierten la simulación en avisos con ubicación.
use crate::model::*;
use crate::sim::Sim;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    pub fn rank(self) -> u8 {
        match self {
            Severity::Error => 0,
            Severity::Warn => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warn => "Aviso",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    pub msg: String,
    pub ids: Vec<String>,
    pub holes: Vec<Hole>,
}

fn nuevo(
    c: &Circuito,
    code: &'static str,
    severity: Severity,
    msg: String,
    ids: Vec<String>,
) -> Issue {
    let holes = ids
        .iter()
        .flat_map(|id| c.find(id).and_then(|x| x.holes().ok()).unwrap_or_default())
        .collect();
    Issue {
        code,
        severity,
        msg,
        ids,
        holes,
    }
}

/// Busca ids de componentes mencionados en un texto de la simulación.
fn ids_en(c: &Circuito, msg: &str) -> Vec<String> {
    c.componentes
        .iter()
        .filter(|x| !x.id.is_empty() && msg.contains(&x.id))
        .map(|x| x.id.clone())
        .collect()
}

pub fn check(c: &Circuito, s: &Sim) -> Vec<Issue> {
    let mut issues = Vec::new();

    for l in &s.leds {
        if l.id.contains('.') {
            continue; // los segmentos internos del display no llevan resistencia propia
        }
        if l.ma > 20.0 {
            issues.push(nuevo(
                c,
                "LED_SIN_R",
                Severity::Error,
                format!(
                    "{} recibe {:.0} mA (máximo recomendado 20 mA): pon una resistencia de 330 Ω en serie.",
                    l.id, l.ma
                ),
                vec![l.id.clone()],
            ));
        }
    }

    for ch in &s.chips {
        if !ch.powered {
            issues.push(nuevo(
                c,
                "CI_SIN_VCC",
                Severity::Error,
                format!(
                    "{} ({}) no tiene alimentación: lleva el pin {} (VCC) al +5 V y el pin {} (GND) a GND.",
                    ch.id,
                    ch.modelo,
                    chip_info(&ch.modelo).map(|i| i.vcc).unwrap_or(14),
                    chip_info(&ch.modelo).map(|i| i.gnd).unwrap_or(7)
                ),
                vec![ch.id.clone()],
            ));
        }
    }

    let mut salidas: HashMap<usize, Vec<(String, u8, Hole)>> = HashMap::new();
    for ch in &s.chips {
        let Some(comp) = c.find(&ch.id) else { continue };
        let Ok(hs) = comp.holes() else { continue };
        let Some(info) = chip_info(&ch.modelo) else {
            continue;
        };
        for (_, out) in info.gates {
            let h = hs[*out as usize - 1];
            if let Some(net) = s.net_of(h) {
                salidas
                    .entry(net)
                    .or_default()
                    .push((ch.id.clone(), *out, h));
            }
        }
    }
    for v in salidas.values() {
        if v.len() < 2 {
            continue;
        }
        let detalle: Vec<String> = v
            .iter()
            .map(|(id, pin, _)| format!("{id} pin {pin}"))
            .collect();
        let mut ids: Vec<String> = Vec::new();
        for (id, _, _) in v {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        issues.push(Issue {
            code: "SALIDA_DOBLE",
            severity: Severity::Error,
            msg: format!(
                "{} comparten la misma red: dos salidas de CI no pueden ir juntas.",
                detalle.join(" y ")
            ),
            ids,
            holes: v.iter().map(|(_, _, h)| *h).collect(),
        });
    }

    for comp in c.componentes.iter().filter(|x| x.tipo == Tipo::Display) {
        let Ok(hs) = comp.holes() else { continue };
        if hs.len() < 8 {
            continue;
        }
        let idxs: &[usize] = if hs.len() > 8 { &[7, 8] } else { &[7] };
        let nets: Vec<usize> = idxs.iter().filter_map(|i| s.net_of(hs[*i])).collect();
        if nets.is_empty() || nets.iter().any(|n| !s.floating[*n]) {
            continue;
        }
        issues.push(nuevo(
            c,
            "DISPLAY_COMUN",
            Severity::Error,
            format!(
                "{}: el común (cátodo) no llega a GND. Conéctalo al riel negativo.",
                comp.id
            ),
            vec![comp.id.clone()],
        ));
    }

    for e in &s.errors {
        if e.to_lowercase().contains("cortocircuito") {
            issues.push(nuevo(c, "CORTO", Severity::Error, e.clone(), ids_en(c, e)));
        } else {
            issues.push(nuevo(c, "SIM", Severity::Error, e.clone(), ids_en(c, e)));
        }
    }

    for w in &s.warnings {
        if w.contains("al aire") {
            let id = w.split(':').next().unwrap_or("").trim().to_string();
            let ids = if c.find(&id).is_some() {
                vec![id]
            } else {
                ids_en(c, w)
            };
            issues.push(nuevo(
                c,
                "ENTRADA_FLOTANTE",
                Severity::Error,
                w.clone(),
                ids,
            ));
        } else if w.contains("no tiene alimentación") {
            continue; // ya está como CI_SIN_VCC
        } else if w.contains("recibe") && w.contains("mA") {
            continue; // ya está como LED_SIN_R
        } else {
            issues.push(nuevo(c, "SIM", Severity::Warn, w.clone(), ids_en(c, w)));
        }
    }

    issues.sort_by_key(|i| (i.severity.rank(), i.code));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim;

    fn circuito(componentes: Vec<Comp>) -> Circuito {
        Circuito {
            nombre: "prueba".into(),
            componentes,
            ..Default::default()
        }
    }

    fn chip(id: &str, pin1: &str, modelo: &str) -> Comp {
        let mut c = Comp::nuevo(id, Tipo::Chip, &[pin1]);
        c.modelo = Some(modelo.into());
        c
    }

    fn issues(c: &Circuito) -> Vec<Issue> {
        check(c, &sim::simulate(c))
    }

    fn tiene(c: &Circuito, code: &str) -> bool {
        issues(c).iter().any(|i| i.code == code)
    }

    #[test]
    fn led_directo_a_la_fuente_es_error() {
        let c = circuito(vec![
            Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
            Comp::nuevo("D1", Tipo::Led, &["t+5", "t-5"]),
        ]);
        assert!(tiene(&c, "LED_SIN_R"), "{:?}", issues(&c));
    }

    #[test]
    fn led_con_resistencia_esta_bien() {
        let c = circuito(vec![
            Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
            Comp::nuevo("R1", Tipo::Resistencia, &["t+5", "a5"]).v(330.0),
            Comp::nuevo("D1", Tipo::Led, &["b5", "t-5"]),
        ]);
        let v = issues(&c);
        assert!(v.is_empty(), "{v:?}");
    }

    #[test]
    fn chip_sin_alimentacion_es_error() {
        let c = circuito(vec![chip("U1", "f12", "74HC08")]);
        assert!(tiene(&c, "CI_SIN_VCC"));
    }

    #[test]
    fn entrada_al_aire_es_error() {
        let c = circuito(vec![
            Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
            Comp::nuevo("W1", Tipo::Cable, &["t+12", "d12"]),
            Comp::nuevo("W2", Tipo::Cable, &["g18", "b-18"]),
            Comp::nuevo("W3", Tipo::Cable, &["t-2", "b-2"]),
            Comp::nuevo("R1", Tipo::Resistencia, &["f14", "a14"]).v(10000.0),
            chip("U1", "f12", "74HC08"),
        ]);
        assert!(tiene(&c, "ENTRADA_FLOTANTE"), "{:?}", issues(&c));
    }

    #[test]
    fn dos_salidas_en_la_misma_red_es_error() {
        let c = circuito(vec![
            Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
            Comp::nuevo("W1", Tipo::Cable, &["t+12", "d12"]),
            Comp::nuevo("W2", Tipo::Cable, &["g18", "b-18"]),
            Comp::nuevo("W3", Tipo::Cable, &["t-2", "b-2"]),
            Comp::nuevo("W4", Tipo::Cable, &["t+20", "d20"]),
            Comp::nuevo("W5", Tipo::Cable, &["g26", "b-26"]),
            Comp::nuevo("W6", Tipo::Cable, &["f14", "f22"]),
            chip("U1", "f12", "74HC08"),
            chip("U2", "f20", "74HC08"),
        ]);
        assert!(tiene(&c, "SALIDA_DOBLE"), "{:?}", issues(&c));
    }

    #[test]
    fn display_sin_comun_es_error() {
        let c = circuito(vec![Comp::nuevo("DS1", Tipo::Display, &["c20"])]);
        assert!(tiene(&c, "DISPLAY_COMUN"), "{:?}", issues(&c));
    }

    #[test]
    fn corto_es_error() {
        let c = circuito(vec![
            Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
            Comp::nuevo("W1", Tipo::Cable, &["t+5", "t-5"]),
        ]);
        assert!(tiene(&c, "CORTO"), "{:?}", issues(&c));
    }
}
