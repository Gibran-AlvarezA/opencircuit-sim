//! Dibujo "físico" de la protoboard y los componentes vistos desde arriba.
use crate::instr::bandas;
use crate::model::*;
use crate::sim::Sim;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2,
};
use std::collections::HashSet;

const GROOVE: Color32 = Color32::from_rgb(214, 208, 195);

#[derive(Clone, Copy)]
pub struct View {
    pub o: Pos2,
    pub s: f32,
    pub tableros: u8,
    pub columnas: i32,
    /// Desplazamiento de cada placa (unidades), para placas movidas a mano.
    pub off: [Vec2; 16],
}

impl View {
    pub fn fit(r: Rect, tableros: u8, columnas: i32, posiciones: &[[f32; 2]]) -> View {
        let (x0, y0, x1, y1) = ext(tableros, columnas, posiciones);
        let s = ((r.width() - 16.0) / (x1 - x0))
            .min((r.height() - 70.0) / (y1 - y0))
            .max(4.0);
        View {
            o: pos2(
                r.center().x - (x0 + x1) / 2.0 * s,
                r.center().y - (y0 + y1) / 2.0 * s,
            ),
            s,
            tableros,
            columnas,
            off: [Vec2::ZERO; 16],
        }
    }
    /// Vista con el origen corrido a la placa `b`: dibuja esa placa con coordenadas sin mover.
    pub fn board_view(&self, b: u8) -> View {
        let d = self.off[(b.max(1) as usize - 1).min(15)];
        View {
            o: self.o + d * self.s,
            off: [Vec2::ZERO; 16],
            ..*self
        }
    }
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.o.x + x * self.s, self.o.y + y * self.s)
    }
    pub fn h(&self, h: Hole) -> Pos2 {
        let (x, y) = hole_xy(h);
        self.board_view(h.board()).p(x, y)
    }
    /// Rectángulo (en pantalla) de la placa `b`.
    pub fn board_rect(&self, b: u8) -> Rect {
        let bx = (b.saturating_sub(1)) as f32 * BOARD_OFFSET;
        let v = self.board_view(b);
        Rect::from_min_max(v.p(bx, 0.0), v.p(bx + board_width(self.columnas), 19.0))
    }
    pub fn inv(&self, p: Pos2) -> (f32, f32) {
        ((p.x - self.o.x) / self.s, (p.y - self.o.y) / self.s)
    }
    pub fn nearest_hole(&self, p: Pos2) -> Option<Hole> {
        Hole::all(self.tableros, self.columnas)
            .into_iter()
            .map(|h| (h, self.h(h).distance(p)))
            .filter(|(_, d)| *d < 0.5 * self.s)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0)
    }
}

pub fn ext(tableros: u8, columnas: i32, posiciones: &[[f32; 2]]) -> (f32, f32, f32, f32) {
    // Encuadra cada placa donde está (también si la movieron a mano), no donde iría sin mover.
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for b in 1..=tableros.max(1) {
        let o = posiciones
            .get(b as usize - 1)
            .copied()
            .unwrap_or([0.0, 0.0]);
        let bx = (b - 1) as f32 * BOARD_OFFSET + o[0];
        x0 = x0.min(bx - 7.0);
        x1 = x1.max(bx + board_width(columnas) + 0.6);
        y0 = y0.min(o[1] - 0.9);
        y1 = y1.max(o[1] + 19.6);
    }
    (x0, y0, x1, y1)
}

/// Color de una red con etiquetas, como texto "#rrggbb" para guardarlo en un cable.
pub fn net_hex(name: &str) -> String {
    let c = net_color(name);
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Nombre de color más parecido (para las instrucciones de armado).
pub fn nombre_color(name: &str) -> String {
    if !name.trim().starts_with('#') {
        return name.to_string();
    }
    let c = color(name);
    let d = |x: Color32| {
        (0..3)
            .map(|i| (x.to_array()[i] as i32 - c.to_array()[i] as i32).pow(2))
            .sum::<i32>()
    };
    WIRE_COLORS
        .iter()
        .chain(["violeta", "gris", "marrón"].iter())
        .min_by_key(|n| d(color(n)))
        .unwrap()
        .to_string()
}

pub fn color(name: &str) -> Color32 {
    let t = name.trim();
    if let Some(h) = t.strip_prefix('#') {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            if h.len() == 6 {
                return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
            }
        }
    }
    match t.to_lowercase().as_str() {
        "rojo" => Color32::from_rgb(215, 40, 40),
        "verde" => Color32::from_rgb(40, 185, 70),
        "amarillo" => Color32::from_rgb(240, 205, 40),
        "azul" => Color32::from_rgb(40, 110, 230),
        "blanco" => Color32::from_rgb(238, 238, 238),
        "negro" => Color32::from_rgb(35, 35, 35),
        "naranja" => Color32::from_rgb(240, 135, 30),
        "violeta" | "morado" => Color32::from_rgb(140, 60, 200),
        "gris" => Color32::from_rgb(130, 130, 130),
        "marrón" | "marron" => Color32::from_rgb(120, 70, 30),
        "dorado" => Color32::from_rgb(200, 160, 60),
        _ => Color32::from_rgb(150, 150, 150),
    }
}

fn shade(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
    )
}

pub fn mat(p: &Painter, r: Rect) {
    p.rect_filled(r, 0.0, Color32::from_rgb(26, 66, 50));
    let st = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 14));
    let mut x = r.left();
    while x < r.right() {
        p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], st);
        x += 24.0;
    }
    let mut y = r.top();
    while y < r.bottom() {
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], st);
        y += 24.0;
    }
}

pub fn board(p: &Painter, v: &View, hl: &HashSet<Hole>) {
    let s = v.s;
    let gray = Color32::from_rgb(120, 115, 105);
    let red = Color32::from_rgb(210, 50, 50);
    let blue = Color32::from_rgb(40, 90, 200);
    let v0 = v;
    for board in 1..=v0.tableros.max(1) {
        let vb = v0.board_view(board);
        let v = &vb;
        let bx = (board.saturating_sub(1)) as f32 * BOARD_OFFSET;
        let bw = board_width(v.columnas);
        p.rect_filled(
            Rect::from_min_max(v.p(bx + 0.25, 0.3), v.p(bx + bw + 0.25, 19.3)),
            0.6 * s,
            Color32::from_black_alpha(100),
        );
        p.rect_filled(
            Rect::from_min_max(v.p(bx, 0.0), v.p(bx + bw, 19.0)),
            0.5 * s,
            Color32::from_rgb(242, 238, 228),
        );
        p.rect_filled(
            Rect::from_min_max(v.p(bx, 9.15), v.p(bx + bw, 9.85)),
            0.0,
            GROOVE,
        );
        for (y, c) in [(0.4, red), (2.6, blue), (16.4, blue), (18.6, red)] {
            p.line_segment(
                [v.p(bx + 1.5, y), v.p(bx + v.columnas as f32 + 1.5, y)],
                Stroke::new(0.12 * s, c),
            );
        }
        for (y, t, c) in [
            (1.0, "+", red),
            (2.0, "-", blue),
            (17.0, "-", blue),
            (18.0, "+", red),
        ] {
            for x in [bx + 0.75, bx + v.columnas as f32 + 2.25] {
                p.text(
                    v.p(x, y),
                    Align2::CENTER_CENTER,
                    t,
                    FontId::proportional(0.9 * s),
                    c,
                );
            }
        }
        for row in 0..10u8 {
            let (_, y) = hole_xy(Hole::Main { board, row, col: 1 });
            for x in [bx + 0.75, bx + v.columnas as f32 + 2.25] {
                p.text(
                    v.p(x, y),
                    Align2::CENTER_CENTER,
                    ((b'a' + row) as char).to_string(),
                    FontId::proportional(0.55 * s),
                    gray,
                );
            }
        }
        for col in 1..=v.columnas {
            if s < 10.0 && col != 1 && col % 5 != 0 {
                continue;
            }
            for y in [3.0, 16.0] {
                p.text(
                    v.p(bx + col as f32 + 1.0, y),
                    Align2::CENTER_CENTER,
                    col.to_string(),
                    FontId::proportional(0.42 * s),
                    gray,
                );
            }
        }
        let g = p.layout_no_wrap(
            format!("PLACA {board}"),
            FontId::proportional(0.52 * s),
            Color32::from_rgb(190, 225, 205),
        );
        let r = Rect::from_center_size(
            v.p(bx + bw / 2.0, -0.55),
            g.size() + vec2(0.5 * s, 0.16 * s),
        );
        p.rect_filled(r, 0.2 * s, Color32::from_rgba_unmultiplied(20, 30, 26, 205));
        p.rect_stroke(
            r,
            0.2 * s,
            Stroke::new(0.05 * s, Color32::from_rgb(90, 130, 110)),
            egui::StrokeKind::Inside,
        );
        p.galley(r.center() - g.size() / 2.0, g, Color32::WHITE);
        for col in 1..=v.columnas {
            for row in 0..10u8 {
                hole_dot(p, v, Hole::Main { board, row, col }, hl, s);
            }
            for (top, plus) in [(true, true), (true, false), (false, false), (false, true)] {
                hole_dot(
                    p,
                    v,
                    Hole::Rail {
                        board,
                        top,
                        plus,
                        col,
                    },
                    hl,
                    s,
                );
            }
        }
    }
}

fn hole_dot(p: &Painter, v: &View, h: Hole, hl: &HashSet<Hole>, s: f32) {
    let c = v.h(h);
    if hl.contains(&h) {
        p.circle_filled(
            c,
            0.45 * s,
            Color32::from_rgba_unmultiplied(40, 200, 90, 150),
        );
    }
    p.rect_filled(
        Rect::from_center_size(c, vec2(0.34 * s, 0.34 * s)),
        0.06 * s,
        Color32::from_rgb(72, 68, 62),
    );
}

#[derive(Clone, Copy)]
pub struct Look {
    pub selected: bool,
    pub current: bool,
    pub focus: bool,
    pub t: f64,
}

pub fn fuente_box(c: &Comp, idx: usize) -> Rect {
    if let Some(q) = c.puntos.first() {
        return Rect::from_min_size(pos2(q[0], q[1]), vec2(5.2, 5.0));
    }
    let y0 = 1.0 + idx as f32 * 6.2;
    Rect::from_min_max(pos2(-6.6, y0), pos2(-1.4, y0 + 5.0))
}

fn fuente_terms(v: &View, c: &Comp, idx: usize) -> (Pos2, Pos2) {
    let b = fuente_box(c, idx);
    (
        v.p(b.right() - 0.7, b.top() + 1.5),
        v.p(b.right() - 0.7, b.top() + 3.6),
    )
}

pub fn wire_ctrl(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let len = d.length();
    if len < 1.0 {
        return a;
    }
    let mut n = vec2(d.y, -d.x) / len;
    if n.y > 0.0 {
        n = -n;
    }
    a + d * 0.5 + n * (len * 0.22)
}

fn bez_pts(a: Pos2, b: Pos2) -> Vec<Pos2> {
    let c = wire_ctrl(a, b);
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            pos2(
                u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
            )
        })
        .collect()
}

/// Recorrido de un cable: curva automática, o tramos rectos por sus puntos de quiebre.
pub fn cable_pts(v: &View, c: &Comp, hs: &[Hole]) -> Vec<Pos2> {
    let (a, b) = (v.h(hs[0]), v.h(hs[1]));
    if c.puntos.is_empty() {
        return bez_pts(a, b);
    }
    std::iter::once(a)
        .chain(c.puntos.iter().map(|q| v.p(q[0], q[1])))
        .chain(std::iter::once(b))
        .collect()
}

fn wire(p: &Painter, a: Pos2, b: Pos2, col: Color32, s: f32) {
    wire_pts(p, bez_pts(a, b), col, s);
}

fn wire_pts(p: &Painter, pts: Vec<Pos2>, col: Color32, s: f32) {
    let (a, b) = (pts[0], *pts.last().unwrap());
    let edge = if col.r() < 60 && col.g() < 60 {
        Color32::from_rgb(10, 10, 10)
    } else {
        shade(col, 0.55)
    };
    let shadow: Vec<Pos2> = pts.iter().map(|q| *q + vec2(0.06 * s, 0.10 * s)).collect();
    p.add(Shape::line(
        shadow,
        Stroke::new(0.42 * s, Color32::from_black_alpha(70)),
    ));
    p.add(Shape::line(pts.clone(), Stroke::new(0.4 * s, edge)));
    p.add(Shape::line(pts.clone(), Stroke::new(0.28 * s, col)));
    // Uniones redondeadas: el codo no queda en punta.
    for q in pts.iter().skip(1).take(pts.len().saturating_sub(2)) {
        p.circle_filled(*q, 0.2 * s, edge);
    }
    for q in pts.iter().skip(1).take(pts.len().saturating_sub(2)) {
        p.circle_filled(*q, 0.14 * s, col);
    }
    let shine: Vec<Pos2> = pts.iter().map(|q| *q + vec2(0.0, -0.06 * s)).collect();
    p.add(Shape::line(
        shine,
        Stroke::new(0.07 * s, Color32::from_white_alpha(70)),
    ));
    for e in [a, b] {
        p.circle_filled(e, 0.17 * s, shade(col, 0.45));
        p.circle_filled(e, 0.1 * s, col);
    }
}

fn quad(p: &Painter, c: Pos2, u: Vec2, hl: f32, hw: f32, fill: Color32, stroke: Stroke) {
    let n = vec2(-u.y, u.x);
    let pts = vec![
        c - u * hl - n * hw,
        c + u * hl - n * hw,
        c + u * hl + n * hw,
        c - u * hl + n * hw,
    ];
    p.add(Shape::convex_polygon(pts, fill, stroke));
}

fn dir(a: Pos2, b: Pos2) -> (Vec2, f32) {
    let d = b - a;
    let len = d.length().max(0.001);
    (d / len, len)
}

pub fn led_dome(a: Pos2, b: Pos2, s: f32) -> Pos2 {
    a + (b - a) * 0.5 + vec2(0.0, -0.55 * s)
}

fn label(p: &Painter, at: Pos2, t: &str, s: f32) {
    let g = p.layout_no_wrap(
        t.to_string(),
        FontId::proportional(0.42 * s),
        Color32::WHITE,
    );
    let r = Rect::from_center_size(at, g.size() + vec2(0.25 * s, 0.05 * s));
    p.rect_filled(
        r,
        0.15 * s,
        Color32::from_rgba_unmultiplied(20, 20, 30, 190),
    );
    p.galley(r.center() - g.size() / 2.0, g, Color32::WHITE);
}

pub fn net_color(name: &str) -> Color32 {
    if name.eq_ignore_ascii_case("GND") {
        return Color32::from_rgb(95, 95, 95);
    }
    let mut h = 0u32;
    for b in name.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    const PALETTE: [Color32; 10] = [
        Color32::from_rgb(40, 120, 220),
        Color32::from_rgb(210, 80, 40),
        Color32::from_rgb(40, 160, 100),
        Color32::from_rgb(180, 60, 170),
        Color32::from_rgb(200, 150, 30),
        Color32::from_rgb(30, 150, 180),
        Color32::from_rgb(150, 80, 200),
        Color32::from_rgb(220, 100, 120),
        Color32::from_rgb(90, 160, 50),
        Color32::from_rgb(160, 120, 60),
    ];
    PALETTE[(h as usize) % PALETTE.len()]
}

fn net_tag(p: &Painter, v: &View, c: &Comp, h: Hole) {
    let at = v.h(h);
    let txt = c.red();
    let col = c
        .color
        .as_deref()
        .map(color)
        .unwrap_or_else(|| net_color(&txt));
    // De lejos las etiquetas se tapan entre sí: queda solo el punto de color.
    if v.s < 9.0 {
        p.circle_filled(at, (0.22 * v.s).max(1.5), col);
        p.circle_stroke(
            at,
            (0.22 * v.s).max(1.5),
            Stroke::new(1.0_f32, Color32::WHITE),
        );
        return;
    }
    let g = p.layout_no_wrap(
        txt,
        FontId::proportional((0.55 * v.s).clamp(8.0, 15.0)),
        Color32::WHITE,
    );
    let center = at + vec2(0.0, -0.55 * v.s - g.size().y * 0.5 - 2.0);
    let tag = Rect::from_center_size(center, g.size() + vec2(8.0, 4.0));
    p.line_segment(
        [at, pos2(at.x, tag.bottom())],
        Stroke::new((0.08 * v.s).max(1.0), col),
    );
    p.rect_filled(
        tag,
        3.0,
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 225),
    );
    p.rect_stroke(
        tag,
        3.0,
        Stroke::new(1.0_f32, Color32::from_black_alpha(90)),
        egui::StrokeKind::Inside,
    );
    p.galley(tag.center() - g.size() / 2.0, g, Color32::WHITE);
}

fn display_body(v: &View, hs: &[Hole], s: f32) -> Rect {
    let x0 = hs[0..7].iter().map(|h| v.h(*h).x).fold(f32::MAX, f32::min);
    let x1 = hs[0..7].iter().map(|h| v.h(*h).x).fold(f32::MIN, f32::max);
    let y = v.h(hs[0]).y;
    Rect::from_min_max(
        pos2(x0 - 0.9 * s, y - 2.9 * s),
        pos2(x1 + 0.9 * s, y - 0.35 * s),
    )
}

fn draw_display(p: &Painter, v: &View, c: &Comp, hs: &[Hole], sim: &Sim, s: f32) {
    let body = display_body(v, hs, s);
    for h in hs {
        p.line_segment(
            [v.h(*h), pos2(v.h(*h).x, body.bottom())],
            Stroke::new(0.11 * s, Color32::from_rgb(175, 175, 182)),
        );
    }
    p.rect_filled(
        body.translate(vec2(0.12 * s, 0.16 * s)),
        0.18 * s,
        Color32::from_black_alpha(110),
    );
    p.rect_filled(body, 0.18 * s, Color32::from_rgb(28, 28, 32));
    p.rect_stroke(
        body,
        0.18 * s,
        Stroke::new(0.06 * s, Color32::from_rgb(90, 90, 100)),
        egui::StrokeKind::Inside,
    );
    let segs = sim.display(&c.id).map(|d| d.segments).unwrap_or([false; 7]);
    let cx = body.center().x;
    let cy = body.center().y;
    let w = (body.width() * 0.22).clamp(0.35 * s, 0.72 * s);
    let h = (body.height() * 0.32).clamp(0.55 * s, 1.0 * s);
    let (xl, xr) = (cx - w, cx + w);
    let (yt, ym, yb) = (cy - h, cy, cy + h);
    let on = Color32::from_rgb(255, 90, 55);
    let off = Color32::from_rgb(52, 46, 42);
    let seg = |a: Pos2, b: Pos2, lit: bool| {
        p.line_segment(
            [a, b],
            Stroke::new((0.13 * s).max(2.0), if lit { on } else { off }),
        );
    };
    seg(pos2(xl, yt), pos2(xr, yt), segs[0]); // a
    seg(pos2(xr, yt), pos2(xr, ym), segs[1]); // b
    seg(pos2(xr, ym), pos2(xr, yb), segs[2]); // c
    seg(pos2(xl, yb), pos2(xr, yb), segs[3]); // d
    seg(pos2(xl, ym), pos2(xl, yb), segs[4]); // e
    seg(pos2(xl, yt), pos2(xl, ym), segs[5]); // f
    seg(pos2(xl, ym), pos2(xr, ym), segs[6]); // g
    p.text(
        body.left_top() + vec2(0.3 * s, 0.25 * s),
        Align2::LEFT_TOP,
        &c.id,
        FontId::proportional(0.42 * s),
        Color32::from_rgb(170, 170, 180),
    );
    if s > 9.0 {
        for (i, h) in hs.iter().enumerate() {
            let t = if i < 7 {
                "abcdefg".chars().nth(i).unwrap().to_string()
            } else {
                "C".into()
            };
            p.text(
                v.h(*h) + vec2(0.0, 0.45 * s),
                Align2::CENTER_CENTER,
                t,
                FontId::proportional(0.34 * s),
                Color32::from_rgb(120, 120, 130),
            );
        }
    }
}

pub fn comp(p: &Painter, v: &View, c: &Comp, hs: &[Hole], sim: &Sim, look: Look, fidx: usize) {
    let vb;
    let v = if matches!(c.tipo, Tipo::Chip | Tipo::Display) {
        vb = v.board_view(hs[0].board()); // usan alturas fijas de la placa
        &vb
    } else {
        v
    };
    let s = v.s;
    let glow = if look.current {
        (0.55 + 0.45 * (look.t * 5.0).sin()) as f32
    } else {
        1.0
    };
    let yel = Color32::from_rgba_unmultiplied(255, 215, 0, (200.0 * glow) as u8);
    // resaltado bajo la pieza
    if look.selected || look.current {
        if c.tipo == Tipo::Chip {
            let (x0, x1) = (v.h(hs[0]).x - 0.9 * s, v.h(hs[6]).x + 0.9 * s);
            p.rect_filled(
                Rect::from_min_max(pos2(x0, v.p(0.0, 7.6).y), pos2(x1, v.p(0.0, 11.4).y)),
                0.3 * s,
                yel,
            );
        } else if c.tipo == Tipo::Fuente {
            let b = fuente_box(c, fidx);
            p.rect_filled(
                Rect::from_min_max(
                    v.p(b.left() - 0.3, b.top() - 0.3),
                    v.p(b.right() + 0.3, b.bottom() + 0.3),
                ),
                0.4 * s,
                yel,
            );
        } else if c.tipo == Tipo::Display {
            let b = display_body(v, hs, s);
            p.rect_filled(
                Rect::from_min_max(
                    pos2(b.left() - 0.3 * s, b.top() - 0.3 * s),
                    pos2(b.right() + 0.3 * s, b.bottom() + 0.3 * s),
                ),
                0.4 * s,
                yel,
            );
        } else if c.tipo == Tipo::Cable {
            p.add(Shape::line(cable_pts(v, c, hs), Stroke::new(0.8 * s, yel)));
        } else if c.tipo == Tipo::Etiqueta {
            p.circle_filled(v.h(hs[0]), 0.55 * s, yel);
        } else if hs.len() >= 2 {
            p.line_segment([v.h(hs[0]), v.h(hs[1])], Stroke::new(1.3 * s, yel));
        }
    }
    let lead = Stroke::new(0.11 * s, Color32::from_rgb(175, 175, 182));
    match c.tipo {
        Tipo::Cable => {
            let ws = if look.focus { s * 1.12 } else { s };
            wire_pts(p, cable_pts(v, c, hs), color(&c.color_or("rojo")), ws);
            if look.selected {
                for q in &c.puntos {
                    let h = v.p(q[0], q[1]);
                    p.circle_filled(h, 0.3 * s, Color32::WHITE);
                    p.circle_stroke(h, 0.3 * s, Stroke::new(0.08 * s, Color32::BLACK));
                }
            }
        }
        Tipo::Resistencia => {
            let (a, b) = (v.h(hs[0]), v.h(hs[1]));
            p.line_segment([a, b], lead);
            let (u, len) = dir(a, b);
            let mid = a + (b - a) * 0.5;
            let hl = (len * 0.36).clamp(0.5 * s, 1.15 * s);
            let hw = 0.3 * s;
            let body = Color32::from_rgb(222, 196, 140);
            for e in [-1.0, 1.0] {
                p.circle_filled(mid + u * hl * e, hw * 1.15, body);
            }
            quad(p, mid, u, hl, hw * 0.9, body, Stroke::NONE);
            for (k, name) in bandas(c.ohmios()).iter().enumerate() {
                let off = [-0.62, -0.3, 0.02, 0.62][k] * hl;
                quad(
                    p,
                    mid + u * off,
                    u,
                    0.08 * s,
                    hw * if k == 3 { 0.9 } else { 1.05 },
                    color(name),
                    Stroke::NONE,
                );
            }
            let n = vec2(-u.y, u.x);
            let n = if n.y > 0.0 { -n } else { n };
            label(p, mid + n * 0.85 * s, &c.id, s);
        }
        Tipo::Led => {
            let (a, b) = (v.h(hs[0]), v.h(hs[1]));
            let d = led_dome(a, b, s);
            p.line_segment([a, d], lead);
            p.line_segment([b, d], lead);
            let col = color(&c.color_or("rojo"));
            let st = sim.led(&c.id);
            let on = st.is_some_and(|x| x.on);
            let r = 0.62 * s;
            if on {
                let br = (st.unwrap().ma / 12.0).clamp(0.3, 1.4) as f32;
                for k in (1..=6).rev() {
                    p.circle_filled(
                        d,
                        r * (1.0 + k as f32 * 0.45 * br),
                        Color32::from_rgba_unmultiplied(
                            col.r(),
                            col.g(),
                            col.b(),
                            (18.0 * br) as u8,
                        ),
                    );
                }
                p.circle_filled(d, r, col);
                p.circle_filled(
                    d,
                    r * 0.55,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 170),
                );
            } else {
                p.circle_filled(d, r, shade(col, 0.45));
                p.circle_stroke(d, r, Stroke::new(0.06 * s, shade(col, 0.3)));
                p.circle_filled(
                    d + vec2(-0.2 * s, -0.2 * s),
                    r * 0.22,
                    Color32::from_white_alpha(90),
                );
            }
            let (u, _) = dir(a, b);
            let n = vec2(-u.y, u.x);
            let fc = d + u * r * 0.92;
            p.line_segment(
                [fc - n * r * 0.55, fc + n * r * 0.55],
                Stroke::new(0.12 * s, shade(col, 0.25)),
            );
            p.text(
                a + vec2(0.0, 0.55 * s),
                Align2::CENTER_CENTER,
                "+",
                FontId::proportional(0.55 * s),
                Color32::from_rgb(200, 40, 40),
            );
            label(p, d + vec2(0.0, -1.05 * s), &c.id, s);
            if st.is_some_and(|x| x.ma > 25.0) {
                p.text(
                    d + vec2(0.8 * s, -0.6 * s),
                    Align2::CENTER_CENTER,
                    "!",
                    FontId::proportional(0.9 * s),
                    Color32::RED,
                );
            }
        }
        Tipo::Interruptor => {
            let (a, b) = (v.h(hs[0]), v.h(hs[1]));
            p.line_segment([a, b], lead);
            let (u, _) = dir(a, b);
            let mid = a + (b - a) * 0.5;
            quad(
                p,
                mid,
                u,
                0.95 * s,
                0.5 * s,
                Color32::from_rgb(35, 80, 170),
                Stroke::new(0.05 * s, Color32::from_rgb(20, 40, 90)),
            );
            quad(
                p,
                mid,
                u,
                0.6 * s,
                0.17 * s,
                Color32::from_rgb(15, 25, 50),
                Stroke::NONE,
            );
            let k = if c.cerrado { 0.3 } else { -0.3 };
            quad(
                p,
                mid + u * k * s,
                u,
                0.26 * s,
                0.32 * s,
                Color32::from_rgb(230, 230, 230),
                Stroke::new(0.04 * s, Color32::GRAY),
            );
            let (txt, tc) = if c.cerrado {
                ("ON", Color32::from_rgb(30, 160, 60))
            } else {
                ("OFF", Color32::from_rgb(200, 50, 50))
            };
            p.text(
                mid + vec2(0.0, 0.95 * s),
                Align2::CENTER_CENTER,
                txt,
                FontId::proportional(0.5 * s),
                tc,
            );
            label(p, mid + vec2(0.0, -0.95 * s), &c.id, s);
        }
        Tipo::Chip => {
            let x0 = v.h(hs[0]).x - 0.6 * s;
            let x1 = v.h(hs[6]).x + 0.6 * s;
            let (yt, yb) = (v.p(0.0, 8.35).y, v.p(0.0, 10.65).y);
            for h in hs {
                let hp = v.h(*h);
                let (ya, yz) = if hp.y > v.p(0.0, 9.5).y {
                    (yb, hp.y + 0.15 * s)
                } else {
                    (hp.y - 0.15 * s, yt)
                };
                p.rect_filled(
                    Rect::from_min_max(
                        pos2(hp.x - 0.17 * s, ya.min(yz)),
                        pos2(hp.x + 0.17 * s, ya.max(yz)),
                    ),
                    0.0,
                    Color32::from_rgb(190, 190, 196),
                );
                if let Some((vv, fl)) = sim.volt_at(*h) {
                    if !fl && vv > 2.5 {
                        p.circle_filled(
                            pos2(hp.x, (ya + yz) / 2.0),
                            0.1 * s,
                            Color32::from_rgb(255, 70, 70),
                        );
                    }
                }
            }
            let body = Rect::from_min_max(pos2(x0, yt), pos2(x1, yb));
            p.rect_filled(body, 0.12 * s, Color32::from_rgb(32, 32, 35));
            p.circle_filled(pos2(x0, body.center().y), 0.32 * s, GROOVE);
            p.circle_filled(
                pos2(x0 + 0.5 * s, yb - 0.4 * s),
                0.12 * s,
                Color32::from_rgb(85, 85, 90),
            );
            let m = c.modelo.clone().unwrap_or_default();
            p.text(
                body.center() + vec2(0.3 * s, 0.0),
                Align2::CENTER_CENTER,
                &m,
                FontId::monospace(0.6 * s),
                Color32::from_rgb(225, 225, 225),
            );
            p.text(
                pos2(x1 - 0.25 * s, yt + 0.4 * s),
                Align2::RIGHT_CENTER,
                &c.id,
                FontId::proportional(0.42 * s),
                Color32::from_rgb(150, 150, 150),
            );
            let powered = sim.chips.iter().any(|x| x.id == c.id && x.powered);
            if !powered {
                p.text(
                    pos2(x1 - 0.25 * s, yb - 0.4 * s),
                    Align2::RIGHT_CENTER,
                    "sin energía",
                    FontId::proportional(0.38 * s),
                    Color32::from_rgb(230, 150, 60),
                );
            }
        }
        Tipo::Display => draw_display(p, v, c, hs, sim, s),
        Tipo::Etiqueta => net_tag(p, v, c, hs[0]),
        Tipo::Fuente => {
            let b = fuente_box(c, fidx);
            let r = Rect::from_min_max(v.p(b.left(), b.top()), v.p(b.right(), b.bottom()));
            p.rect_filled(
                r.translate(vec2(0.15 * s, 0.2 * s)),
                0.4 * s,
                Color32::from_black_alpha(110),
            );
            p.rect_filled(r, 0.4 * s, Color32::from_rgb(48, 52, 62));
            p.rect_stroke(
                r,
                0.4 * s,
                Stroke::new(0.06 * s, Color32::from_rgb(90, 95, 110)),
                egui::StrokeKind::Inside,
            );
            p.text(
                v.p(b.left() + 0.4, b.top() + 0.8),
                Align2::LEFT_CENTER,
                "FUENTE",
                FontId::proportional(0.55 * s),
                Color32::from_rgb(200, 200, 210),
            );
            p.text(
                v.p(b.left() + 0.4, b.top() + 2.2),
                Align2::LEFT_CENTER,
                format!("{:.1} V", c.voltios()),
                FontId::monospace(0.9 * s),
                Color32::from_rgb(120, 230, 140),
            );
            p.text(
                v.p(b.left() + 0.4, b.top() + 3.6),
                Align2::LEFT_CENTER,
                &c.id,
                FontId::proportional(0.5 * s),
                Color32::from_rgb(160, 160, 170),
            );
            p.circle_filled(
                v.p(b.left() + 0.6, b.bottom() - 0.6),
                0.18 * s,
                Color32::from_rgb(60, 220, 90),
            );
            let (tp, tm) = fuente_terms(v, c, fidx);
            for (t, col, sym) in [
                (tp, Color32::from_rgb(200, 40, 40), "+"),
                (tm, Color32::from_rgb(25, 25, 25), "-"),
            ] {
                p.circle_filled(t, 0.38 * s, col);
                p.text(
                    t + vec2(-0.85 * s, 0.0),
                    Align2::CENTER_CENTER,
                    sym,
                    FontId::proportional(0.8 * s),
                    Color32::WHITE,
                );
            }
            wire(p, tp, v.h(hs[0]), Color32::from_rgb(215, 40, 40), s);
            wire(p, tm, v.h(hs[1]), Color32::from_rgb(35, 35, 35), s);
        }
    }
    if look.current {
        for h in hs {
            p.circle_stroke(v.h(*h), 0.42 * s, Stroke::new(0.12 * s, yel));
        }
    }
}

/// Fantasma del chip que se va a colocar.
pub fn chip_ghost(p: &Painter, v: &View, board: u8, col: i32) {
    let v = &v.board_view(board);
    let s = v.s;
    let bx = (board.saturating_sub(1)) as f32 * BOARD_OFFSET;
    let r = Rect::from_min_max(
        pos2(
            v.p(bx + col as f32 + 1.0, 0.0).x - 0.6 * s,
            v.p(0.0, 8.35).y,
        ),
        pos2(
            v.p(bx + col as f32 + 7.0, 0.0).x + 0.6 * s,
            v.p(0.0, 10.65).y,
        ),
    );
    p.rect_filled(
        r,
        0.12 * s,
        Color32::from_rgba_unmultiplied(30, 30, 30, 120),
    );
    p.rect_stroke(
        r,
        0.12 * s,
        Stroke::new(2.0_f32, Color32::YELLOW),
        egui::StrokeKind::Outside,
    );
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

pub fn poly_dist(p: Pos2, pts: &[Pos2]) -> f32 {
    pts.windows(2)
        .map(|w| seg_dist(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

/// Distancia del ratón a la pieza (None si está lejos).
pub fn hit(v: &View, c: &Comp, hs: &[Hole], m: Pos2, fidx: usize) -> Option<f32> {
    let vb;
    let v = if matches!(c.tipo, Tipo::Chip | Tipo::Display) {
        vb = v.board_view(hs[0].board());
        &vb
    } else {
        v
    };
    let s = v.s;
    let d = match c.tipo {
        Tipo::Chip => {
            let r = Rect::from_min_max(
                pos2(v.h(hs[0]).x - 0.6 * s, v.p(0.0, 8.35).y),
                pos2(v.h(hs[6]).x + 0.6 * s, v.p(0.0, 10.65).y),
            );
            if r.contains(m) {
                0.0
            } else {
                f32::MAX
            }
        }
        Tipo::Display => {
            let r = display_body(v, hs, s);
            if r.contains(m) {
                0.0
            } else {
                hs.iter()
                    .map(|h| m.distance(v.h(*h)))
                    .fold(f32::MAX, f32::min)
                    - 0.3 * s
            }
        }
        Tipo::Etiqueta => m.distance(v.h(hs[0])) - 0.4 * s,
        Tipo::Fuente => {
            let b = fuente_box(c, fidx);
            if Rect::from_min_max(v.p(b.left(), b.top()), v.p(b.right(), b.bottom())).contains(m) {
                0.0
            } else {
                let (tp, tm) = fuente_terms(v, c, fidx);
                poly_dist(m, &bez_pts(tp, v.h(hs[0]))).min(poly_dist(m, &bez_pts(tm, v.h(hs[1]))))
            }
        }
        Tipo::Cable => poly_dist(m, &cable_pts(v, c, hs)),
        Tipo::Led => m.distance(led_dome(v.h(hs[0]), v.h(hs[1]), s)) - 0.4 * s,
        _ => seg_dist(m, v.h(hs[0]), v.h(hs[1])) - 0.1 * s,
    };
    (d < 0.42 * s).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_encuadre_incluye_placas_movidas() {
        // Con la placa 2 corrida a la izquierda y hacia abajo, el encuadre la incluye.
        let (x0, y0, x1, y1) = ext(2, 30, &[[0.0, 0.0], [-120.0, 20.0]]);
        assert!(x0 <= -60.0, "{x0}");
        assert!(y1 >= 39.0, "{y1}");
        assert!(x1 > board_width(30), "{x1}");
        assert_eq!(y0, -0.9);
    }
}
