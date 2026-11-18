//! Simulación en corriente continua: análisis nodal + LEDs por tramos + lógica 74HC.
use crate::model::*;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write;

const R_LED: f64 = 10.0;
const R_OUT: f64 = 50.0;
const G_OFF: f64 = 1e-9;

#[derive(Clone, Debug)]
pub struct LedState {
    pub id: String,
    pub on: bool,
    pub ma: f64,
}

#[derive(Clone, Debug)]
pub struct DisplayState {
    pub id: String,
    pub segments: [bool; 7],
    pub dp: bool,
    pub text: char,
}

#[derive(Clone, Debug)]
pub struct ChipState {
    pub id: String,
    pub modelo: String,
    pub powered: bool,
    pub outputs: Vec<(u8, bool)>,
}

#[derive(Default, Clone)]
pub struct Sim {
    pub net_of_strip: HashMap<Strip, usize>,
    pub volt: Vec<f64>,
    pub floating: Vec<bool>,
    pub leds: Vec<LedState>,
    pub displays: Vec<DisplayState>,
    pub chips: Vec<ChipState>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl Sim {
    pub fn net_of(&self, h: Hole) -> Option<usize> {
        self.net_of_strip.get(&h.strip()).copied()
    }
    /// (voltios, flotante)
    pub fn volt_at(&self, h: Hole) -> Option<(f64, bool)> {
        self.net_of(h).map(|n| (self.volt[n], self.floating[n]))
    }
    pub fn led(&self, id: &str) -> Option<&LedState> {
        self.leds.iter().find(|l| l.id == id)
    }
    pub fn display(&self, id: &str) -> Option<&DisplayState> {
        self.displays.iter().find(|d| d.id == id)
    }
}

pub fn led_vf(color: &str) -> f64 {
    match color {
        "azul" | "blanco" => 3.0,
        "verde" => 2.1,
        "amarillo" => 2.0,
        _ => 1.8,
    }
}

/// Carácter que forma el display con esos segmentos (a..g).
pub fn display_char(segs: &[bool; 7]) -> char {
    let [a, b, c, d, e, f, g] = *segs;
    match (a, b, c, d, e, f, g) {
        (true, true, true, true, true, true, false) => '0',
        (false, true, true, false, false, false, false) => '1',
        (true, true, false, true, true, false, true) => '2',
        (true, true, true, true, false, false, true) => '3',
        (false, true, true, false, false, true, true) => '4',
        (true, false, true, true, false, true, true) => '5',
        (true, false, true, true, true, true, true) => '6',
        (true, true, true, false, false, false, false) => '7',
        (true, true, true, true, true, true, true) => '8',
        (true, true, true, true, false, true, true) => '9',
        (true, false, false, true, true, true, true) => 'E',
        (false, false, false, false, false, false, false) => ' ',
        _ => '?',
    }
}

type Gates = Vec<(usize, Vec<usize>, u8, Vec<u8>)>; // (red salida, redes entrada, pin salida, pines entrada)

struct LedCalc {
    id: String,
    a: usize,
    k: usize,
    vf: f64,
    /// (índice del display, segmento 0..7) si este LED es interno de un display.
    display: Option<(usize, usize)>,
}

struct DisplayCalc {
    id: String,
}

pub fn simulate(c: &Circuito) -> Sim {
    let mut sim = Sim::default();
    let mut parsed = Vec::new();
    for comp in &c.componentes {
        if let Ok(h) = comp.holes() {
            parsed.push((comp, h));
        }
    }
    sim.errors = c.validate();

    let mut strip_ix: HashMap<Strip, usize> = HashMap::new();
    for (_, hs) in &parsed {
        for h in hs {
            let n = strip_ix.len();
            strip_ix.entry(h.strip()).or_insert(n);
        }
    }
    let mut uf: Vec<usize> = (0..strip_ix.len()).collect();
    fn find(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x {
            uf[x] = uf[uf[x]];
            x = uf[x];
        }
        x
    }
    for (comp, hs) in &parsed {
        if comp.tipo == Tipo::Cable || (comp.tipo == Tipo::Interruptor && comp.cerrado) {
            let a = find(&mut uf, strip_ix[&hs[0].strip()]);
            let b = find(&mut uf, strip_ix[&hs[1].strip()]);
            uf[a] = b;
        }
    }
    // Las etiquetas con el mismo nombre son la misma red, aunque estén en placas distintas.
    let mut labels: HashMap<String, Vec<usize>> = HashMap::new();
    for (comp, hs) in &parsed {
        if comp.tipo == Tipo::Etiqueta {
            let name = comp.red().to_uppercase();
            if !name.is_empty() {
                labels
                    .entry(name)
                    .or_default()
                    .push(strip_ix[&hs[0].strip()]);
            }
        }
    }
    for group in labels.values() {
        if let Some((&first, rest)) = group.split_first() {
            for &x in rest {
                let a = find(&mut uf, first);
                let b = find(&mut uf, x);
                uf[a] = b;
            }
        }
    }
    let mut sv: Vec<(Strip, usize)> = strip_ix.iter().map(|(s, i)| (*s, *i)).collect();
    sv.sort_by_key(|x| x.1);
    let mut root_net: HashMap<usize, usize> = HashMap::new();
    let mut nets: HashMap<Strip, usize> = HashMap::new();
    for (s, i) in sv {
        let r = find(&mut uf, i);
        let n = root_net.len();
        nets.insert(s, *root_net.entry(r).or_insert(n));
    }
    let n = root_net.len();
    let net = |h: &Hole| nets[&h.strip()];
    let mut pins_in_net = vec![0usize; n];
    for (_, hs) in &parsed {
        for h in hs {
            pins_in_net[net(h)] += 1;
        }
    }

    // Fuentes: fijan el voltaje de sus redes.
    let mut fixed: Vec<Option<f64>> = vec![None; n];
    let mut supply = false;
    for (comp, hs) in &parsed {
        if comp.tipo != Tipo::Fuente {
            continue;
        }
        let (p, m) = (net(&hs[0]), net(&hs[1]));
        if p == m {
            sim.errors.push(format!("¡Cortocircuito! El + y el - de {} están unidos directamente (por cables o un interruptor cerrado). En la vida real la fuente se calentaría o se dañaría.", comp.id));
            continue;
        }
        for (x, v) in [(m, 0.0), (p, comp.voltios())] {
            match fixed[x] {
                Some(o) if (o - v).abs() > 1e-6 => sim.errors.push(format!(
                    "¡Cortocircuito! {} une puntos que deberían tener voltajes distintos ({:.1} V y {:.1} V).",
                    comp.id, o, v
                )),
                _ => fixed[x] = Some(v),
            }
        }
        supply = true;
    }
    if !supply {
        sim.warnings.push(
            "No hay fuente de alimentación: añade una «Fuente» conectada a los rieles.".into(),
        );
    }

    let mut res = Vec::new();
    let mut leds: Vec<LedCalc> = Vec::new();
    let mut displays: Vec<DisplayCalc> = Vec::new();
    let mut chips: Vec<(String, &ChipInfo, bool, f64, Gates)> = Vec::new();
    for (comp, hs) in &parsed {
        match comp.tipo {
            Tipo::Resistencia => res.push((net(&hs[0]), net(&hs[1]), 1.0 / comp.ohmios())),
            Tipo::Led => leds.push(LedCalc {
                id: comp.id.clone(),
                a: net(&hs[0]),
                k: net(&hs[1]),
                vf: led_vf(&comp.color_or("rojo")),
                display: None,
            }),
            Tipo::Display => {
                let di = displays.len();
                let seg = [
                    net(&hs[0]),
                    net(&hs[1]),
                    net(&hs[2]),
                    net(&hs[3]),
                    net(&hs[4]),
                    net(&hs[5]),
                    net(&hs[6]),
                ];
                // Los dos pines «común» del módulo están unidos por dentro: sirve cualquiera.
                // shortcut: si se conectan a nets distintos se ignora el conflicto.
                let common = if hs.len() > 8 {
                    let (c1, c2) = (net(&hs[7]), net(&hs[8]));
                    if fixed[c1].is_some() {
                        c1
                    } else {
                        c2
                    }
                } else {
                    net(&hs[7])
                };
                displays.push(DisplayCalc {
                    id: comp.id.clone(),
                });
                for (si, s) in seg.iter().enumerate() {
                    leds.push(LedCalc {
                        id: format!("{}.{}", comp.id, "abcdefg".chars().nth(si).unwrap()),
                        a: *s,
                        k: common,
                        vf: 1.8,
                        display: Some((di, si)),
                    });
                }
                if hs.len() > 8 {
                    leds.push(LedCalc {
                        id: format!("{}.dp", comp.id),
                        a: net(&hs[9]),
                        k: common,
                        vf: 1.8,
                        display: Some((di, 7)),
                    });
                }
            }
            Tipo::Chip => {
                let info = chip_info(comp.modelo.as_deref().unwrap_or("")).unwrap();
                let (vcc, gnd) = (net(&hs[13]), net(&hs[6]));
                let col = hs[0].col();
                let (mut powered, mut vdd) = (false, 0.0);
                match (fixed[vcc], fixed[gnd]) {
                    (Some(v), Some(g)) if v - g >= 2.0 => {
                        powered = true;
                        vdd = v - g;
                        if vdd > 6.0 {
                            sim.warnings.push(format!("{}: los chips 74HC aguantan hasta 6 V; con {:.1} V se dañaría.", comp.id, vdd));
                        }
                    }
                    (Some(v), Some(g)) if g - v >= 2.0 => sim.errors.push(format!(
                        "¡{} está alimentado al revés! El pin 14 (VCC) va al + y el pin 7 (GND) al -. Así se dañaría.",
                        comp.id
                    )),
                    _ => sim.warnings.push(format!(
                        "{} no tiene alimentación: conecta su pin 14 (VCC, en e{}) al + y su pin 7 (GND, en f{}) al -.",
                        comp.id,
                        col,
                        col + 6
                    )),
                }
                let gates = info
                    .gates
                    .iter()
                    .map(|(ins, out)| {
                        (
                            net(&hs[*out as usize - 1]),
                            ins.iter().map(|p| net(&hs[*p as usize - 1])).collect(),
                            *out,
                            ins.to_vec(),
                        )
                    })
                    .collect();
                chips.push((comp.id.clone(), info, powered, vdd, gates));
            }
            _ => {}
        }
    }

    // Redes "al aire": sin camino conductor hasta una fuente o una salida activa.
    let mut adj = vec![Vec::new(); n];
    for r in &res {
        adj[r.0].push(r.1);
        adj[r.1].push(r.0);
    }
    for l in &leds {
        adj[l.a].push(l.k);
        adj[l.k].push(l.a);
    }
    let mut floating = vec![true; n];
    let mut q = VecDeque::new();
    let seed = |i: usize, f: &mut Vec<bool>, q: &mut VecDeque<usize>| {
        if f[i] {
            f[i] = false;
            q.push_back(i);
        }
    };
    for (i, f) in fixed.iter().enumerate() {
        if f.is_some() {
            seed(i, &mut floating, &mut q);
        }
    }
    for ch in &chips {
        if ch.2 {
            for g in &ch.4 {
                seed(g.0, &mut floating, &mut q);
            }
        }
    }
    while let Some(x) = q.pop_front() {
        for &y in &adj[x] {
            seed(y, &mut floating, &mut q);
        }
    }

    let mut outs: Vec<Vec<bool>> = chips.iter().map(|c| vec![false; c.4.len()]).collect();
    let mut led_on = vec![false; leds.len()];
    let mut v = vec![0.0; n];
    let mut stable = false;
    for _ in 0..40 {
        let mut norton = Vec::new();
        for (ci, ch) in chips.iter().enumerate() {
            if ch.2 {
                for (gi, g) in ch.4.iter().enumerate() {
                    norton.push((g.0, 1.0 / R_OUT, if outs[ci][gi] { ch.3 } else { 0.0 }));
                }
            }
        }
        for _ in 0..30 {
            let mut cond = res.clone();
            let mut inj = Vec::new();
            for (i, l) in leds.iter().enumerate() {
                if led_on[i] {
                    let g = 1.0 / R_LED;
                    cond.push((l.a, l.k, g));
                    inj.push((l.a, g * l.vf));
                    inj.push((l.k, -g * l.vf));
                } else {
                    cond.push((l.a, l.k, G_OFF));
                }
            }
            v = solve(n, &fixed, &cond, &inj, &norton);
            let mut changed = false;
            for (i, l) in leds.iter().enumerate() {
                let d = v[l.a] - v[l.k];
                if led_on[i] && d < l.vf {
                    led_on[i] = false;
                    changed = true;
                } else if !led_on[i] && d > l.vf + 1e-4 {
                    led_on[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut new = outs.clone();
        for (ci, ch) in chips.iter().enumerate() {
            if !ch.2 {
                continue;
            }
            for (gi, g) in ch.4.iter().enumerate() {
                let ins: Vec<bool> =
                    g.1.iter()
                        .map(|&nn| !floating[nn] && v[nn] > ch.3 * 0.5)
                        .collect();
                new[ci][gi] = ch.1.gate.eval(&ins);
            }
        }
        if new == outs {
            stable = true;
            break;
        }
        outs = new;
    }
    if !stable {
        sim.warnings.push("El circuito oscila (sus salidas cambian sin parar): revisa si una salida vuelve a su propia entrada.".into());
    }

    for (ci, ch) in chips.iter().enumerate() {
        if ch.2 {
            for g in &ch.4 {
                if pins_in_net[g.0] < 2 {
                    continue; // salida sin usar: no importa
                }
                for (k, &nn) in g.1.iter().enumerate() {
                    if floating[nn] {
                        sim.warnings.push(format!("{}: la entrada del pin {} está «al aire» (no llega ni al + ni al -). Da resultados al azar: añade una resistencia de 10 kΩ hacia el - (pull-down).", ch.0, g.3[k]));
                    } else if v[nn] > 0.3 * ch.3 && v[nn] < 0.7 * ch.3 {
                        sim.warnings.push(format!("{}: el pin {} recibe {:.2} V, un voltaje intermedio: el chip no sabe si es 0 o 1.", ch.0, g.3[k], v[nn]));
                    }
                }
            }
        }
        sim.chips.push(ChipState {
            id: ch.0.clone(),
            modelo: ch.1.modelo.into(),
            powered: ch.2,
            outputs: ch.4.iter().zip(&outs[ci]).map(|(g, o)| (g.2, *o)).collect(),
        });
    }

    for (comp, hs) in &parsed {
        if comp.tipo == Tipo::Display && hs.len() == 8 {
            sim.warnings.push(format!(
                "{}: usa el display antiguo de 8 pines; el módulo real de 1.2\" se declara con \"pines\":[\"c20\"]",
                comp.id
            ));
        }
    }

    let mut seg_on = vec![[false; 7]; displays.len()];
    let mut dp_on = vec![false; displays.len()];
    for (i, l) in leds.iter().enumerate() {
        let ma = if led_on[i] {
            (v[l.a] - v[l.k] - l.vf) / R_LED * 1000.0
        } else {
            0.0
        };
        if let Some((di, si)) = l.display {
            if si < 7 {
                seg_on[di][si] = ma > 0.1;
            } else {
                dp_on[di] = ma > 0.1;
            }
            continue; // los segmentos no se listan como LEDs sueltos
        }
        if ma > 60.0 {
            sim.warnings.push(format!(
                "{} recibe {:.0} mA: ¡se quemaría! Pon una resistencia en serie (330 Ω).",
                l.id, ma
            ));
        } else if ma > 25.0 {
            sim.warnings.push(format!(
                "{} recibe {:.0} mA, más de lo recomendado (20 mA): usa una resistencia mayor.",
                l.id, ma
            ));
        }
        if !led_on[i] && v[l.k] - v[l.a] > 1.5 && !floating[l.a] && !floating[l.k] {
            sim.warnings.push(format!(
                "{} parece estar al revés: la pata larga (ánodo) debe ir hacia el +.",
                l.id
            ));
        }
        sim.leds.push(LedState {
            id: l.id.clone(),
            on: ma > 0.1,
            ma,
        });
    }
    for (di, d) in displays.iter().enumerate() {
        sim.displays.push(DisplayState {
            id: d.id.clone(),
            segments: seg_on[di],
            dp: dp_on[di],
            text: display_char(&seg_on[di]),
        });
    }

    sim.volt = v;
    sim.floating = floating;
    sim.net_of_strip = nets;
    sim
}

fn solve(
    n: usize,
    fixed: &[Option<f64>],
    cond: &[(usize, usize, f64)],
    inj: &[(usize, f64)],
    norton: &[(usize, f64, f64)],
) -> Vec<f64> {
    let mut idx = vec![usize::MAX; n];
    let mut m = 0;
    for i in 0..n {
        if fixed[i].is_none() {
            idx[i] = m;
            m += 1;
        }
    }
    let mut a = vec![vec![0.0; m]; m];
    let mut b = vec![0.0; m];
    for (i, row) in a.iter_mut().enumerate() {
        row[i] += G_OFF; // fuga mínima a tierra: evita sistemas sin solución con redes sueltas
    }
    for &(p, q, g) in cond {
        match (fixed[p], fixed[q]) {
            (None, None) => {
                let (i, j) = (idx[p], idx[q]);
                a[i][i] += g;
                a[j][j] += g;
                a[i][j] -= g;
                a[j][i] -= g;
            }
            (None, Some(vq)) => {
                a[idx[p]][idx[p]] += g;
                b[idx[p]] += g * vq;
            }
            (Some(vp), None) => {
                a[idx[q]][idx[q]] += g;
                b[idx[q]] += g * vp;
            }
            _ => {}
        }
    }
    for &(p, cur) in inj {
        if fixed[p].is_none() {
            b[idx[p]] += cur;
        }
    }
    for &(p, g, vv) in norton {
        if fixed[p].is_none() {
            a[idx[p]][idx[p]] += g;
            b[idx[p]] += g * vv;
        }
    }
    gauss(&mut a, &mut b);
    (0..n)
        .map(|i| fixed[i].unwrap_or_else(|| b[idx[i]]))
        .collect()
}

fn gauss(a: &mut [Vec<f64>], b: &mut [f64]) {
    let m = b.len();
    for col in 0..m {
        let piv = (col..m)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        if d.abs() < 1e-30 {
            continue;
        }
        let prow = a[col].clone();
        for r in col + 1..m {
            let f = a[r][col] / d;
            if f == 0.0 {
                continue;
            }
            for k in col..m {
                a[r][k] -= f * prow[k];
            }
            b[r] -= f * b[col];
        }
    }
    for col in (0..m).rev() {
        let mut s = b[col];
        for k in col + 1..m {
            s -= a[col][k] * b[k];
        }
        b[col] = if a[col][col].abs() < 1e-30 {
            0.0
        } else {
            s / a[col][col]
        };
    }
}

pub struct Tabla {
    pub sw: Vec<String>,
    pub leds: Vec<String>,
    pub displays: Vec<String>,
    pub filas: Vec<(Vec<bool>, Vec<bool>)>,
    /// Carácter de cada display en cada fila.
    pub disp_filas: Vec<Vec<char>>,
}

/// Tabla de verdad probando todas las posiciones de los interruptores (hasta 5).
pub fn tabla(c: &Circuito) -> Option<Tabla> {
    let sw: Vec<usize> = (0..c.componentes.len())
        .filter(|&i| c.componentes[i].tipo == Tipo::Interruptor)
        .collect();
    let leds: Vec<String> = c
        .componentes
        .iter()
        .filter(|x| x.tipo == Tipo::Led)
        .map(|x| x.id.clone())
        .collect();
    let displays: Vec<String> = c
        .componentes
        .iter()
        .filter(|x| x.tipo == Tipo::Display)
        .map(|x| x.id.clone())
        .collect();
    if sw.is_empty() || sw.len() > 5 || (leds.is_empty() && displays.is_empty()) {
        return None;
    }
    let mut filas = Vec::new();
    let mut disp_filas = Vec::new();
    for mask in 0..(1u32 << sw.len()) {
        let mut cc = c.clone();
        let st: Vec<bool> = (0..sw.len())
            .map(|k| mask & (1 << (sw.len() - 1 - k)) != 0)
            .collect();
        for (k, &i) in sw.iter().enumerate() {
            cc.componentes[i].cerrado = st[k];
        }
        let s = simulate(&cc);
        filas.push((
            st,
            leds.iter()
                .map(|l| s.led(l).is_some_and(|x| x.on))
                .collect(),
        ));
        disp_filas.push(
            displays
                .iter()
                .map(|d| s.display(d).map(|x| x.text).unwrap_or(' '))
                .collect(),
        );
    }
    Some(Tabla {
        sw: sw.iter().map(|&i| c.componentes[i].id.clone()).collect(),
        leds,
        displays,
        filas,
        disp_filas,
    })
}

/// Resumen en texto (para la línea de comandos y los agentes de IA).
pub fn resumen(c: &Circuito) -> String {
    let s = simulate(c);
    let mut o = String::new();
    let _ = writeln!(
        o,
        "Circuito «{}»: {} componentes, {} placa(s) de {} columnas",
        c.nombre,
        c.componentes.len(),
        c.tableros,
        c.columnas
    );
    if !s.errors.is_empty() {
        let _ = writeln!(o, "\nERRORES:");
        for e in &s.errors {
            let _ = writeln!(o, "- {e}");
        }
    }
    if !s.leds.is_empty() {
        let _ = writeln!(o, "\nLEDs:");
        for l in &s.leds {
            let col = c
                .find(&l.id)
                .map(|x| x.color_or("rojo"))
                .unwrap_or_default();
            if l.on {
                let _ = writeln!(o, "- {} ({}): ENCENDIDO, {:.1} mA", l.id, col, l.ma);
            } else {
                let _ = writeln!(o, "- {} ({}): apagado", l.id, col);
            }
        }
    }
    for d in &s.displays {
        let segs: String = "abcdefg"
            .chars()
            .zip(d.segments)
            .filter(|(_, on)| *on)
            .map(|(c, _)| c)
            .collect();
        let _ = writeln!(
            o,
            "- {}: muestra «{}» (segmentos: {})",
            d.id,
            d.text,
            if segs.is_empty() { "ninguno" } else { &segs }
        );
    }
    for ch in &s.chips {
        let outs: Vec<String> = ch
            .outputs
            .iter()
            .map(|(p, v)| format!("pin {}={}", p, *v as u8))
            .collect();
        let _ = writeln!(
            o,
            "- {} {}: {}{}",
            ch.id,
            ch.modelo,
            if ch.powered {
                "alimentado; salidas "
            } else {
                "SIN alimentación"
            },
            if ch.powered {
                outs.join(", ")
            } else {
                String::new()
            }
        );
    }
    for x in c.componentes.iter().filter(|x| x.tipo == Tipo::Interruptor) {
        let _ = writeln!(
            o,
            "- {}: {}",
            x.id,
            if x.cerrado { "cerrado" } else { "abierto" }
        );
    }
    if !s.warnings.is_empty() {
        let _ = writeln!(o, "\nADVERTENCIAS:");
        for w in &s.warnings {
            let _ = writeln!(o, "- {w}");
        }
    }
    if let Some(t) = tabla(c) {
        let mut head: Vec<String> = t.sw.clone();
        head.extend(t.leds.iter().cloned());
        head.extend(t.displays.iter().cloned());
        let _ = writeln!(o, "\nTABLA DE VERDAD ({}):", head.join(" "));
        for (ri, (a, b)) in t.filas.iter().enumerate() {
            let f = |v: &Vec<bool>| {
                v.iter()
                    .map(|x| if *x { "1" } else { "0" })
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let mut line = format!("  {} | {}", f(a), f(b));
            for ch in &t.disp_filas[ri] {
                line.push_str(&format!(" | {ch}"));
            }
            let _ = writeln!(o, "{line}");
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn practica5() -> Circuito {
        ejemplos()
            .into_iter()
            .find(|c| c.nombre.contains("Práctica 5"))
            .expect("falta assets/practica5.json")
    }

    fn con_switches(mut c: Circuito, bits: &[bool]) -> Circuito {
        for (i, b) in bits.iter().enumerate() {
            if let Some(x) = c
                .componentes
                .iter_mut()
                .find(|x| x.id == format!("S{}", i + 1))
            {
                x.cerrado = *b;
            }
        }
        c
    }

    #[test]
    fn display_de_10_pines() {
        use Tipo::*;
        let k = Comp::nuevo;
        let c = Circuito {
            componentes: vec![
                k("F1", Fuente, &["t+1", "t-1"]).v(5.0),
                k("R1", Resistencia, &["t+2", "d24"]).v(330.0),
                k("R2", Resistencia, &["t+3", "h23"]).v(330.0),
                k("W1", Cable, &["t-2", "d22"]),
                k("DS1", Display, &["c20"]),
            ],
            ..Default::default()
        };
        let nombres: Vec<String> = c
            .find("DS1")
            .unwrap()
            .holes()
            .unwrap()
            .iter()
            .map(|h| h.name())
            .collect();
        // Orden lógico a, b, c, d, e, f, g, común 1, común 2, dp.
        assert_eq!(
            nombres,
            ["c23", "c24", "g23", "g21", "g20", "c21", "c20", "c22", "g22", "g24"]
        );
        let s = simulate(&c);
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        let d = s.display("DS1").unwrap();
        assert_eq!(d.segments, [false, true, true, false, false, false, false]);
        assert_eq!(d.text, '1');
        assert!(!d.dp);
        // El segundo común también sirve (los dos están unidos por dentro).
        let mut c2 = c.clone();
        c2.componentes.retain(|x| x.id != "W1");
        c2.componentes.push(k("W2", Cable, &["t-4", "h22"]));
        let s2 = simulate(&c2);
        assert_eq!(s2.display("DS1").unwrap().text, '1', "{:?}", s2.errors);
    }

    #[test]
    fn display_no_cabe_en_la_placa() {
        let c = Comp::nuevo("DS1", Tipo::Display, &["j30"]);
        assert!(c.holes().is_err());
        let c = Comp::nuevo("DS1", Tipo::Display, &["c58"]);
        assert!(c.holes().is_err());
    }

    #[test]
    fn patillaje_7404() {
        let info = chip_info("7404").expect("7404");
        assert_eq!(
            info.gates,
            [
                (&[1u8][..], 2u8),
                (&[3][..], 4),
                (&[5][..], 6),
                (&[9][..], 8),
                (&[11][..], 10),
                (&[13][..], 12),
            ]
        );
        assert!(info.pin_fn(1).contains("entrada"));
        assert!(info.pin_fn(8).contains("salida"));
        assert!(info.pin_fn(9).contains("entrada"));
        assert_eq!(
            info.pinout(),
            "compuerta 1: entrada 1 → salida 2; compuerta 2: entrada 3 → salida 4; compuerta 3: entrada 5 → salida 6; compuerta 4: entrada 9 → salida 8; compuerta 5: entrada 11 → salida 10; compuerta 6: entrada 13 → salida 12"
        );
    }

    #[test]
    fn led_con_resistencia() {
        let s = simulate(&ejemplos()[0]);
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        let l = s.led("D1").unwrap();
        assert!(l.on && (l.ma - 9.4).abs() < 0.3, "{l:?}");
    }

    #[test]
    fn compuerta_and() {
        let c = ejemplos()[1].clone();
        let s = simulate(&c);
        assert!(
            s.errors.is_empty() && s.warnings.is_empty(),
            "{:?} {:?}",
            s.errors,
            s.warnings
        );
        let t = tabla(&c).unwrap();
        let leds: Vec<bool> = t.filas.iter().map(|f| f.1[0]).collect();
        assert_eq!(leds, vec![false, false, false, true]);
    }

    #[test]
    fn inversor_y_cortocircuito() {
        let t = tabla(&ejemplos()[2]).unwrap();
        assert_eq!(
            t.filas.iter().map(|f| f.1[0]).collect::<Vec<_>>(),
            vec![true, false]
        );
        let mut c = ejemplos()[0].clone();
        c.componentes
            .push(Comp::nuevo("W9", Tipo::Cable, &["t+20", "t-20"]));
        assert!(simulate(&c)
            .errors
            .iter()
            .any(|e| e.contains("Cortocircuito")));
    }

    #[test]
    fn entrada_al_aire() {
        let mut c = ejemplos()[1].clone();
        c.componentes.retain(|x| x.id != "R1");
        c.componentes
            .iter_mut()
            .find(|x| x.id == "S1")
            .unwrap()
            .cerrado = false;
        assert!(simulate(&c).warnings.iter().any(|w| w.contains("al aire")));
    }

    fn con_etiquetas(segunda: &str) -> Circuito {
        Circuito {
            nombre: "etiquetas".into(),
            componentes: vec![
                Comp::nuevo("F1", Tipo::Fuente, &["t+1", "t-1"]).v(5.0),
                Comp::nuevo("W1", Tipo::Cable, &["t+5", "a5"]),
                Comp::nuevo("L1", Tipo::Etiqueta, &["b5"]).n("X"),
                Comp::nuevo("L2", Tipo::Etiqueta, &["b10"]).n(segunda),
                Comp::nuevo("R1", Tipo::Resistencia, &["c10", "c11"]).v(470.0),
                Comp::nuevo("D1", Tipo::Led, &["d11", "d12"]),
                Comp::nuevo("W2", Tipo::Cable, &["e12", "t-12"]),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn etiquetas_unen_redes() {
        let s = simulate(&con_etiquetas("X"));
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        assert!(
            s.led("D1").is_some_and(|l| l.on),
            "la etiqueta X debería unir las dos redes"
        );
        let s = simulate(&con_etiquetas("Y"));
        assert!(!s.led("D1").is_some_and(|l| l.on), "Y no debe unir nada");
    }

    #[test]
    fn display_muestra_1_y_e() {
        let c = practica5();
        let s = simulate(&con_switches(c.clone(), &[false, false, false, true, true]));
        let d = s.display("DS1").expect("falta el display DS1");
        assert_eq!(d.text, '1');
        assert_eq!(d.segments, [false, true, true, false, false, false, false]);
        let s = simulate(&con_switches(c, &[false; 5]));
        let d = s.display("DS1").expect("falta el display DS1");
        assert_eq!(d.text, 'E');
        assert_eq!(d.segments, [true, false, false, true, true, true, true]);
    }

    #[test]
    fn practica5_32_casos() {
        let c = practica5();
        let base = simulate(&c);
        assert_eq!(base.chips.len(), 12, "deben ser 12 CI");
        assert!(
            base.chips.iter().all(|ch| ch.powered),
            "algún CI quedó sin alimentación"
        );
        let csv = include_str!("../assets/checklist_32.csv");
        let mut ok = 0;
        for line in csv.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() < 15 {
                continue;
            }
            let bits = [
                f[1] == "1",
                f[2] == "1",
                f[3] == "1",
                f[4] == "1",
                f[5] == "1",
            ];
            let s = simulate(&con_switches(c.clone(), &bits));
            let led_a = s.led("D1").is_some_and(|l| l.on);
            let led_b = s.led("D2").is_some_and(|l| l.on);
            let disp = s.display("DS1").map(|d| d.text).unwrap_or(' ');
            let esperado_a = f[10] == "1";
            let esperado_b = f[11] == "1";
            let esperado_disp = f[12].chars().next().unwrap_or(' ');
            if led_a == esperado_a && led_b == esperado_b && disp == esperado_disp {
                ok += 1;
            } else {
                panic!(
                    "fila {}: A-E {:?}; LED A {} (esperado {}), LED B {} (esperado {}), display '{}' (esperado '{}')",
                    f[0], bits, led_a, esperado_a, led_b, esperado_b, disp, esperado_disp
                );
            }
        }
        assert_eq!(ok, 32, "solo pasaron {ok}/32");
        println!("32/32");
    }

    /// Práctica 6: 16 combinaciones de A-D contra el checklist y los CI alimentados.
    fn check_practica6(nombre: &str, csv: &str, chips: usize) {
        let c = ejemplos()
            .into_iter()
            .find(|c| c.nombre.contains(nombre))
            .expect("falta el JSON de la Práctica 6");
        let base = simulate(&c);
        assert!(base.errors.is_empty(), "{:?}", base.errors);
        assert!(
            !base.warnings.iter().any(|w| w.contains("al aire")),
            "entradas flotantes: {:?}",
            base.warnings
        );
        assert_eq!(base.chips.len(), chips, "deben ser {chips} CI");
        assert!(
            base.chips.iter().all(|ch| ch.powered),
            "algún CI quedó sin alimentación"
        );
        let mut ok = 0;
        for line in csv.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() < 12 {
                continue;
            }
            let bits = [f[1] == "1", f[2] == "1", f[3] == "1", f[4] == "1"];
            let s = simulate(&con_switches(c.clone(), &bits));
            let d = s.display("DS1").expect("falta el display DS1");
            let esperado = [f[5], f[6], f[7], f[8], f[9], f[10], f[11]].map(|x| x == "1");
            assert_eq!(d.segments, esperado, "fila m={} con {bits:?}", f[0]);
            ok += 1;
        }
        assert_eq!(ok, 16, "solo pasaron {ok}/16");
    }

    #[test]
    fn practica6_rumbo_16_casos() {
        check_practica6("rumbo", include_str!("../assets/checklist_rumbo.csv"), 8);
    }

    #[test]
    fn practica6_celular_16_casos() {
        check_practica6(
            "celular",
            include_str!("../assets/checklist_celular.csv"),
            9,
        );
    }
}
