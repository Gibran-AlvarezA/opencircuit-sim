//! Vista de impresion: exporta el circuito a SVG vectorial con tema claro.
//!
//! A diferencia de la vista de la app (las placas van en fila), aqui se **apilan**
//! verticalmente para que la figura aproveche la pagina. Reutiliza la geometria de
//! `draw` (orificios, curvas de cable, cuerpo de chip, display y fuente).
//!
//! Nota: este archivo se mantiene en ASCII a proposito (los textos con acentos se
//! reciben por CLI) para que el SVG no dependa de la codificacion del editor.
use crate::draw;
use crate::instr::bandas;
use crate::model::*;
use crate::sim::Sim;
use eframe::egui::{pos2, vec2, Pos2, Vec2};

/// Opciones de exportacion.
#[derive(Clone, Debug)]
pub struct Cfg {
    /// Solo esta placa (1..=3). `None` = todas (apiladas).
    pub placa: Option<u8>,
    /// Ancho del dibujo en milimetros.
    pub ancho_mm: f32,
    /// Titulo; si es `None` se usa el nombre del circuito.
    pub titulo: Option<String>,
    /// Dibuja la leyenda de colores al pie.
    pub leyenda: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            placa: None,
            ancho_mm: 300.0,
            titulo: None,
            leyenda: true,
        }
    }
}

// ---- paleta de impresion (clara) ----
const FONDO: &str = "#ffffff";
const PLACA: &str = "#f3efe4";
const RANURA: &str = "#e3ddcc";
const HOYO: &str = "#b6ae9c";
const RIEL_ROJO: &str = "#c0392b";
const RIEL_AZUL: &str = "#2f5fa8";
const TEXTO: &str = "#1d2328";
const TEXTO_SUAVE: &str = "#6b7280";
const CHIP: &str = "#20242a";
const CHIP_TEXTO: &str = "#f5f6f7";
const PATA: &str = "#8d939a";
const CUERPO_R: &str = "#ead6ac";
const DISPLAY: &str = "#171a1e";
const SEG_ON: &str = "#ff5a37";
const SEG_OFF: &str = "#3a352f";

/// Separacion vertical entre placas apiladas (unidades de placa).
const GAP: f32 = 7.0;
/// Margen izquierdo extra para la caja de la fuente.
const IZQ: f32 = 9.0;

pub fn hex(c: eframe::egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

fn sombra(c: &str, f: f32) -> String {
    let h = c.trim_start_matches('#');
    if h.len() != 6 {
        return c.to_string();
    }
    let v = u32::from_str_radix(h, 16).unwrap_or(0);
    let (r, g, b) = ((v >> 16) & 255, (v >> 8) & 255, v & 255);
    format!(
        "#{:02x}{:02x}{:02x}",
        (r as f32 * f) as u32,
        (g as f32 * f) as u32,
        (b as f32 * f) as u32
    )
}

struct Lienzo {
    s: String,
}

impl Lienzo {
    fn new() -> Self {
        Self {
            s: String::with_capacity(1 << 20),
        }
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, rx: f32, fill: &str) {
        self.s.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" rx="{rx:.2}" fill="{fill}"/>"#
        ));
    }
    fn rect_st(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rx: f32,
        fill: &str,
        stroke: &str,
        sw: f32,
    ) {
        self.s.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" rx="{rx:.2}" fill="{fill}" stroke="{stroke}" stroke-width="{sw:.2}"/>"#
        ));
    }
    fn line(&mut self, a: Pos2, b: Pos2, color: &str, w: f32) {
        self.s.push_str(&format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{w:.2}" stroke-linecap="round"/>"#,
            a.x, a.y, b.x, b.y, color
        ));
    }
    fn circle(&mut self, c: Pos2, r: f32, fill: &str) {
        self.s.push_str(&format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{r:.2}" fill="{}"/>"#,
            c.x, c.y, fill
        ));
    }
    fn circle_st(&mut self, c: Pos2, r: f32, fill: &str, stroke: &str, sw: f32) {
        self.s.push_str(&format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{r:.2}" fill="{}" stroke="{stroke}" stroke-width="{sw:.2}"/>"#,
            c.x, c.y, fill
        ));
    }
    fn poly(&mut self, pts: &[Pos2], fill: &str, stroke: &str, sw: f32) {
        let d: Vec<String> = pts
            .iter()
            .map(|p| format!("{:.2},{:.2}", p.x, p.y))
            .collect();
        self.s.push_str(&format!(
            r#"<polygon points="{}" fill="{}" stroke="{stroke}" stroke-width="{sw:.2}" stroke-linejoin="round"/>"#,
            d.join(" "),
            fill
        ));
    }
    fn path(&mut self, pts: &[Pos2], color: &str, w: f32, op: f32) {
        if pts.len() < 2 {
            return;
        }
        let mut d = format!("M {:.2} {:.2}", pts[0].x, pts[0].y);
        for p in &pts[1..] {
            d.push_str(&format!(" L {:.2} {:.2}", p.x, p.y));
        }
        self.s.push_str(&format!(
            r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="{w:.2}" stroke-opacity="{op:.2}" stroke-linecap="round" stroke-linejoin="round"/>"#
        ));
    }
    /// Texto horizontal. anchor: 0 = izquierda, 1 = centro, 2 = derecha.
    fn text(&mut self, at: Pos2, txt: &str, size: f32, color: &str, anchor: u8, bold: bool) {
        let a = match anchor {
            1 => "middle",
            2 => "end",
            _ => "start",
        };
        let weight = if bold { " font-weight=\"600\"" } else { "" };
        self.s.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, Arial, sans-serif" font-size="{size:.2}" fill="{color}" text-anchor="{a}"{weight}>{}</text>"#,
            at.x,
            at.y,
            esc(txt)
        ));
    }
    /// Texto vertical (de abajo hacia arriba) para etiquetas de red.
    fn text_vert(&mut self, at: Pos2, txt: &str, size: f32, color: &str, arriba: bool) {
        let rot = if arriba { -90.0 } else { 90.0 };
        self.s.push_str(&format!(
            r#"<g transform="translate({:.2},{:.2}) rotate({rot})"><text x="0" y="{:.2}" font-family="Segoe UI, Arial, sans-serif" font-size="{size:.2}" fill="{color}" text-anchor="start" font-weight="600">{}</text></g>"#,
            at.x,
            at.y,
            size * 0.34,
            esc(txt)
        ));
    }
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Transforma coordenadas de placa a la lamina apilada.
#[derive(Clone, Copy)]
struct Mapa {
    ox: f32,
    oy: f32,
    s: f32,
    /// Placas que se dibujan, en orden vertical.
    slots: [u8; 3],
    n: u8,
}

impl Mapa {
    /// Posicion vertical de la placa `b` (0, 1, 2...), no su numero de placa.
    fn slot(&self, b: u8) -> f32 {
        (0..self.n)
            .find(|k| self.slots[*k as usize] == b)
            .unwrap_or(0) as f32
    }
    fn pb(&self, b: u8, x: f32, y: f32) -> Pos2 {
        let dy = self.slot(b) * (19.0 + GAP);
        pos2(self.ox + (x + IZQ) * self.s, self.oy + (y + dy) * self.s)
    }
    /// Coordenadas de placa (las de `hole_xy`, que ya traen el corrimiento horizontal).
    fn p(&self, x: f32, y: f32) -> Pos2 {
        let b = ((x / BOARD_OFFSET).floor() as i32).max(0) as u8 + 1;
        self.pb(b, x - (b - 1) as f32 * BOARD_OFFSET, y)
    }
    fn h(&self, h: Hole) -> Pos2 {
        let (x, y) = hole_xy(h);
        self.p(x, y)
    }
}

fn quad_pts(c: Pos2, u: Vec2, hl: f32, hw: f32) -> Vec<Pos2> {
    let n = vec2(-u.y, u.x);
    vec![
        c - u * hl - n * hw,
        c + u * hl - n * hw,
        c + u * hl + n * hw,
        c - u * hl + n * hw,
    ]
}

fn dir(a: Pos2, b: Pos2) -> (Vec2, f32) {
    let d = b - a;
    let len = d.length().max(0.001);
    (d / len, len)
}

fn bez(a: Pos2, b: Pos2) -> Vec<Pos2> {
    let d = b - a;
    // Ruteo ortogonal (como un cable tendido en la protoboard): un tramo recto si
    // los orificios comparten fila o columna, y una sola esquina si no.
    if d.x.abs() < 0.35 || d.y.abs() < 0.35 {
        return vec![a, b];
    }
    let codo = if d.x.abs() >= d.y.abs() {
        pos2(b.x, a.y)
    } else {
        pos2(a.x, b.y)
    };
    vec![a, codo, b]
}

fn cable_pts(m: &Mapa, c: &Comp, hs: &[Hole]) -> Vec<Pos2> {
    let (a, b) = (m.h(hs[0]), m.h(hs[1]));
    if c.puntos.is_empty() {
        return bez(a, b);
    }
    let brd = hs[0].board();
    std::iter::once(a)
        .chain(c.puntos.iter().map(|q| {
            let lx = q[0] - (brd.saturating_sub(1)) as f32 * BOARD_OFFSET;
            m.pb(brd, lx, q[1])
        }))
        .chain(std::iter::once(b))
        .collect()
}

/// Exporta el circuito a un SVG.
pub fn exportar(circ: &Circuito, sim: &Sim, cfg: &Cfg) -> String {
    let tableros = circ.tableros.max(1);
    let placas: Vec<u8> = match cfg.placa {
        Some(p) => vec![p],
        None => (1..=tableros).collect(),
    };
    let n = placas.len().max(1);
    let columnas = circ.columnas;

    const W: f32 = 1240.0;
    let cabecera = 96.0;
    let pie = if cfg.leyenda { 78.0 } else { 32.0 };
    let ancho_u = board_width(columnas) + IZQ + 3.0;
    let alto_u = n as f32 * 19.0 + (n as f32 - 1.0) * GAP + 5.0;
    let s = (W - 56.0) / ancho_u;
    let alto_px = cabecera + alto_u * s + pie;
    let m = Mapa {
        ox: 28.0,
        oy: cabecera + 4.0 * s,
        s,
        slots: [placas[0], *placas.get(1).unwrap_or(&placas[0]), *placas.get(2).unwrap_or(&placas[0])],
        n: n as u8,
    };

    let mut l = Lienzo::new();

    // ---------- cabecera ----------
    let titulo = cfg.titulo.clone().unwrap_or_else(|| circ.nombre.clone());
    l.text(pos2(28.0, 44.0), &titulo, 26.0, TEXTO, 0, true);
    let chips = circ.componentes.iter().filter(|c| c.tipo == Tipo::Chip).count();
    let cables = circ.componentes.iter().filter(|c| c.tipo == Tipo::Cable).count();
    let etiquetas = circ
        .componentes
        .iter()
        .filter(|c| c.tipo == Tipo::Etiqueta)
        .count();
    let sub = format!(
        "OpenCircuit  |  {chips} integrados  |  {cables} cables  |  {etiquetas} etiquetas de red  |  5 V  |  {} placa(s)",
        n
    );
    l.text(pos2(28.0, 66.0), &sub, 13.0, TEXTO_SUAVE, 0, false);
    l.line(pos2(28.0, 78.0), pos2(W - 28.0, 78.0), "#e5e7eb", 1.0);

    // ---------- placas ----------
    for (i, &b) in placas.iter().enumerate() {
        let dy = i as f32 * (19.0 + GAP);
        let bw = board_width(columnas);
        let x0 = m.ox + IZQ * s;
        let y0 = m.oy + dy * s;
        let w = bw * s;
        l.rect(x0, y0, w, 19.0 * s, 0.35 * s, PLACA);
        l.rect(x0, y0 + 9.15 * s, w, 0.7 * s, 0.0, RANURA);
        for (y, signo, c) in [
            (1.0, "+", RIEL_ROJO),
            (2.0, "-", RIEL_AZUL),
            (17.0, "-", RIEL_AZUL),
            (18.0, "+", RIEL_ROJO),
        ] {
            l.line(m.pb(b, 0.6, y), m.pb(b, bw - 0.6, y), c, 0.14 * s);
            l.text(m.pb(b, -0.35, y), signo, 0.8 * s, c, 1, true);
        }
        for row in 0..10u8 {
            let (_, y) = hole_xy(Hole::Main {
                board: b,
                row,
                col: 1,
            });
            l.text(
                m.pb(b, -0.4, y - 0.06),
                &((b'a' + row) as char).to_string(),
                0.48 * s,
                TEXTO_SUAVE,
                1,
                false,
            );
        }
        for col in 1..=columnas {
            if col != 1 && col % 5 != 0 {
                continue;
            }
            for y in [3.15, 15.95] {
                l.text(
                    m.pb(b, col as f32 + 1.0, y),
                    &col.to_string(),
                    0.4 * s,
                    TEXTO_SUAVE,
                    1,
                    false,
                );
            }
        }
        for col in 1..=columnas {
            for row in 0..10u8 {
                l.circle(m.h(Hole::Main { board: b, row, col }), 0.15 * s, HOYO);
            }
            for (top, plus) in [(true, true), (true, false), (false, false), (false, true)] {
                l.circle(
                    m.h(Hole::Rail {
                        board: b,
                        top,
                        plus,
                        col,
                    }),
                    0.12 * s,
                    HOYO,
                );
            }
        }
        l.text(
            m.pb(b, bw / 2.0, -1.7),
            &format!("PLACA {b}"),
            0.8 * s,
            "#8a6d1f",
            1,
            true,
        );
    }

    // ---------- piezas ----------
    let mut items: Vec<(&Comp, Vec<Hole>)> = circ
        .componentes
        .iter()
        .filter(|c| !circ.oculta(c))
        .filter_map(|c| c.holes().ok().map(|h| (c, h)))
        .filter(|(c, hs)| {
            hs.first().is_some_and(|h| placas.contains(&h.board()))
                && (!matches!(c.tipo, Tipo::Cable)
                    || hs.iter().all(|h| h.board() == hs[0].board()))
        })
        .collect();
    items.sort_by_key(|(c, _)| match c.tipo {
        Tipo::Fuente => 0,
        Tipo::Cable => 1,
        Tipo::Resistencia | Tipo::Led | Tipo::Interruptor => 2,
        Tipo::Chip | Tipo::Display => 3,
        Tipo::Etiqueta => 4,
    });

    for (c, hs) in &items {
        if c.tipo != Tipo::Cable {
            continue;
        }
        let pts = cable_pts(&m, c, hs);
        let col = hex(draw::color(&c.color_or("rojo")));
        let somb: Vec<Pos2> = pts.iter().map(|q| *q + vec2(0.06 * s, 0.10 * s)).collect();
        l.path(&somb, "#000000", 0.40 * s, 0.13);
        l.path(&pts, &sombra(&col, 0.55), 0.34 * s, 1.0);
        l.path(&pts, &col, 0.22 * s, 1.0);
        for e in [pts[0], *pts.last().unwrap()] {
            l.circle(e, 0.13 * s, &sombra(&col, 0.45));
            l.circle(e, 0.08 * s, &col);
        }
    }

    for (c, hs) in &items {
        if c.tipo == Tipo::Cable {
            continue;
        }
        let b = hs[0].board();
        match c.tipo {
            Tipo::Chip => {
                let x0 = m.h(hs[0]).x - 0.55 * s;
                let x1 = m.h(hs[6]).x + 0.55 * s;
                let yt = m.pb(b, 0.0, 8.35).y;
                let yb = m.pb(b, 0.0, 10.65).y;
                for h in hs.iter() {
                    let hp = m.h(*h);
                    let yy = if hp.y < m.pb(b, 0.0, 9.5).y { yt } else { yb };
                    l.line(pos2(hp.x, hp.y), pos2(hp.x, yy), PATA, 0.15 * s);
                }
                l.rect_st(x0, yt, x1 - x0, yb - yt, 0.14 * s, CHIP, "#0b0d10", 0.03 * s);
                l.circle(pos2(x0, (yt + yb) / 2.0), 0.26 * s, PLACA);
                l.text(
                    pos2((x0 + x1) / 2.0 + 0.2 * s, (yt + yb) / 2.0 + 0.02 * s),
                    &c.modelo.clone().unwrap_or_default(),
                    0.56 * s,
                    CHIP_TEXTO,
                    1,
                    true,
                );
                l.text(
                    pos2(x1 - 0.2 * s, yt + 0.42 * s),
                    &c.id,
                    0.36 * s,
                    "#9aa0a6",
                    2,
                    false,
                );
            }
            Tipo::Resistencia => {
                let (a, bb) = (m.h(hs[0]), m.h(hs[1]));
                l.line(a, bb, PATA, 0.11 * s);
                let (u, len) = dir(a, bb);
                let mid = a + (bb - a) * 0.5;
                let hl = (len * 0.36).clamp(0.45 * s, 1.05 * s);
                let hw = 0.27 * s;
                l.poly(&quad_pts(mid, u, hl, hw * 0.9), CUERPO_R, "#c9ae7a", 0.03 * s);
                for e in [-1.0f32, 1.0] {
                    l.circle(mid + u * hl * e, hw * 1.08, CUERPO_R);
                }
                for (k, name) in bandas(c.ohmios()).iter().enumerate() {
                    let off = [-0.62f32, -0.3, 0.02, 0.62][k] * hl;
                    let col = hex(draw::color(name));
                    l.poly(
                        &quad_pts(
                            mid + u * off,
                            u,
                            0.075 * s,
                            hw * if k == 3 { 0.88 } else { 1.02 },
                        ),
                        &col,
                        &col,
                        0.0,
                    );
                }
                l.text(mid + vec2(0.0, -0.5 * s), &c.id, 0.38 * s, TEXTO, 1, false);
            }
            Tipo::Led => {
                let (a, bb) = (m.h(hs[0]), m.h(hs[1]));
                let d = draw::led_dome(a, bb, s);
                l.line(a, d, PATA, 0.11 * s);
                l.line(bb, d, PATA, 0.11 * s);
                let col = hex(draw::color(&c.color_or("rojo")));
                let on = sim.led(&c.id).is_some_and(|x| x.on);
                let relleno = if on { col.clone() } else { sombra(&col, 0.55) };
                l.circle_st(d, 0.55 * s, &relleno, &sombra(&col, 0.35), 0.04 * s);
                if on {
                    l.circle(d, 0.26 * s, "#ffffffcc");
                } else {
                    l.circle(d + vec2(-0.18 * s, -0.18 * s), 0.12 * s, "#ffffffaa");
                }
                l.text(a + vec2(0.0, 0.5 * s), "+", 0.45 * s, "#c0392b", 1, true);
                l.text(d + vec2(0.0, -0.9 * s), &c.id, 0.38 * s, TEXTO, 1, false);
            }
            Tipo::Interruptor => {
                let (a, bb) = (m.h(hs[0]), m.h(hs[1]));
                l.line(a, bb, PATA, 0.11 * s);
                let (u, _) = dir(a, bb);
                let mid = a + (bb - a) * 0.5;
                l.poly(
                    &quad_pts(mid, u, 0.85 * s, 0.45 * s),
                    "#2f5fa8",
                    "#1b3a6b",
                    0.03 * s,
                );
                l.poly(
                    &quad_pts(mid, u, 0.55 * s, 0.15 * s),
                    "#0f1932",
                    "#0f1932",
                    0.0,
                );
                let k = if c.cerrado { 0.26 } else { -0.26 };
                l.poly(
                    &quad_pts(mid + u * k * s, u, 0.23 * s, 0.29 * s),
                    "#eeeeee",
                    "#9aa0a6",
                    0.02 * s,
                );
                l.text(
                    mid + vec2(0.0, 0.85 * s),
                    if c.cerrado { "ON" } else { "OFF" },
                    0.42 * s,
                    if c.cerrado { "#1e9e50" } else { "#c0392b" },
                    1,
                    true,
                );
                l.text(mid + vec2(0.0, -0.85 * s), &c.id, 0.38 * s, TEXTO, 1, false);
            }
            Tipo::Display => {
                let x0 = hs[0..7].iter().map(|h| m.h(*h).x).fold(f32::MAX, f32::min);
                let x1 = hs[0..7].iter().map(|h| m.h(*h).x).fold(f32::MIN, f32::max);
                let y = m.h(hs[0]).y;
                let top = y - 2.8 * s;
                let bot = y - 0.3 * s;
                for h in hs.iter() {
                    let hp = m.h(*h);
                    l.line(hp, pos2(hp.x, bot), PATA, 0.11 * s);
                }
                l.rect_st(
                    x0 - 0.8 * s,
                    top,
                    (x1 - x0) + 1.6 * s,
                    bot - top,
                    0.16 * s,
                    DISPLAY,
                    "#000000",
                    0.03 * s,
                );
                let segs = sim
                    .display(&c.id)
                    .map(|d| d.segments)
                    .unwrap_or([false; 7]);
                let cx = (x0 + x1) / 2.0;
                let cy = (top + bot) / 2.0;
                let w = ((x1 - x0) * 0.28).clamp(0.3 * s, 0.66 * s);
                let h = ((bot - top) * 0.30).clamp(0.5 * s, 0.9 * s);
                let (xl, xr) = (cx - w, cx + w);
                let (yt2, ym, yb2) = (cy - h, cy, cy + h);
                let seg = |a: Pos2, b: Pos2, lit: bool, l: &mut Lienzo| {
                    l.line(a, b, if lit { SEG_ON } else { SEG_OFF }, 0.12 * s);
                };
                seg(pos2(xl, yt2), pos2(xr, yt2), segs[0], &mut l);
                seg(pos2(xr, yt2), pos2(xr, ym), segs[1], &mut l);
                seg(pos2(xr, ym), pos2(xr, yb2), segs[2], &mut l);
                seg(pos2(xl, yb2), pos2(xr, yb2), segs[3], &mut l);
                seg(pos2(xl, ym), pos2(xl, yb2), segs[4], &mut l);
                seg(pos2(xl, yt2), pos2(xl, ym), segs[5], &mut l);
                seg(pos2(xl, ym), pos2(xr, ym), segs[6], &mut l);
                l.text(
                    pos2(x0 - 0.6 * s, top - 0.15 * s),
                    &c.id,
                    0.38 * s,
                    "#6b7280",
                    0,
                    false,
                );
            }
            Tipo::Etiqueta => {
                let at = m.h(hs[0]);
                let txt = c.red();
                let col = match c.color.as_deref() {
                    Some(n) => hex(draw::color(n)),
                    None => hex(draw::net_color(&txt)),
                };
                // Etiqueta vertical pegada al orificio: legible aunque haya una por columna.
                let arriba = match hs[0] {
                    Hole::Main { row, .. } => row <= 4,
                    Hole::Rail { top, .. } => top,
                };
                let size = (0.62 * s).min(15.0);
                let signo = if arriba { -1.0 } else { 1.0 };
                let base = at + vec2(0.0, signo * 0.55 * s);
                l.line(at, base, &col, 0.05 * s);
                l.circle(at, 0.1 * s, &col);
                l.text_vert(base, &txt, size, &col, arriba);
            }
            Tipo::Fuente => {
                let bx = draw::fuente_box(c, 0);
                let x0 = m.pb(b, bx.left(), bx.top());
                let ancho = (bx.right() - bx.left()) * s;
                let alto = (bx.bottom() - bx.top()) * s;
                l.rect_st(
                    x0.x,
                    x0.y,
                    ancho,
                    alto,
                    0.4 * s,
                    "#2b3038",
                    "#11151a",
                    0.05 * s,
                );
                l.text(
                    pos2(x0.x + 0.4 * s, x0.y + 0.9 * s),
                    "FUENTE",
                    0.55 * s,
                    "#e8eaed",
                    0,
                    true,
                );
                l.text(
                    pos2(x0.x + 0.4 * s, x0.y + 2.2 * s),
                    &format!("{:.1} V", c.voltios()),
                    0.82 * s,
                    "#49d17a",
                    0,
                    false,
                );
                l.text(
                    pos2(x0.x + 0.4 * s, x0.y + 3.4 * s),
                    &c.id,
                    0.45 * s,
                    "#aeb4ba",
                    0,
                    false,
                );
                let tp = pos2(x0.x + ancho - 0.7 * s, x0.y + 1.5 * s);
                let tm = pos2(x0.x + ancho - 0.7 * s, x0.y + 3.6 * s);
                l.circle(tp, 0.32 * s, "#c0392b");
                l.text(tp, "+", 0.6 * s, "#ffffff", 1, true);
                l.circle(tm, 0.32 * s, "#1d2328");
                l.text(tm, "-", 0.6 * s, "#ffffff", 1, true);
                if let Some(h0) = hs.first() {
                    l.path(&[tp, m.h(*h0)], "#c0392b", 0.18 * s, 1.0);
                }
                if let Some(h1) = hs.get(1) {
                    l.path(&[tm, m.h(*h1)], "#1d2328", 0.18 * s, 1.0);
                }
            }
            Tipo::Cable => {}
        }
    }

    // ---------- leyenda ----------
    if cfg.leyenda {
        let y = alto_px - 52.0;
        l.text(pos2(28.0, y), "Colores de red", 12.5, TEXTO, 0, true);
        let mut nets: Vec<String> = Vec::new();
        for c in &circ.componentes {
            match c.tipo {
                Tipo::Etiqueta | Tipo::Cable => nets.push(c.red()),
                _ => {}
            }
        }
        nets.sort();
        nets.dedup();
        let mut x = 28.0;
        let mut row = 0;
        for n in nets.iter().take(64) {
            let w = 24.0 + 6.4 * n.chars().count() as f32;
            if x + w > W - 28.0 {
                x = 28.0;
                row += 1;
            }
            let yy = y + 17.0 + row as f32 * 16.0;
            let col = hex(draw::net_color(n));
            l.line(pos2(x, yy - 3.5), pos2(x + 14.0, yy - 3.5), &col, 3.4);
            l.text(pos2(x + 19.0, yy), n, 10.0, TEXTO_SUAVE, 0, false);
            x += w;
        }
    }

    let mm = cfg.ancho_mm.max(80.0);
    let alto_mm = mm * alto_px / W;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{mm:.1}mm" height="{alto_mm:.1}mm" viewBox="0 0 {W:.2} {alto_px:.2}">
<rect x="0" y="0" width="{W:.2}" height="{alto_px:.2}" fill="{FONDO}"/>
{}
</svg>"#,
        l.s
    )
}
