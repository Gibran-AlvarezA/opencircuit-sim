//! Circuito, protoboard y formato JSON (el mismo que usan los agentes de IA).
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Columnas de una placa normal (valor por defecto).
pub const COLS: i32 = 30;
/// Máximo de columnas que admite el simulador (placas grandes).
pub const MAX_COLS: i32 = 60;
/// Máximo de placas que se pueden poner en fila.
pub const MAX_TABLEROS: u8 = 3;
/// Separación en unidades de dibujo entre el borde de una placa y la siguiente.
pub const BOARD_OFFSET: f32 = 64.0;

/// Un orificio de la protoboard. Filas 0..10 = a..j.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Hole {
    Main {
        board: u8,
        row: u8,
        col: i32,
    },
    Rail {
        board: u8,
        top: bool,
        plus: bool,
        col: i32,
    },
}

/// Grupo de orificios conectados por dentro de la placa.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Strip {
    Column { board: u8, top: bool, col: i32 },
    Rail { board: u8, top: bool, plus: bool },
}

impl Hole {
    /// Acepta "a1", "t+5", "b-12" y, para la segunda o tercera placa, "a1@2", "t+5@3".
    pub fn parse(s: &str) -> Option<Hole> {
        let s = s.trim().to_lowercase();
        let (body, board) = match s.split_once('@') {
            Some((b, n)) => (b, n.parse::<u8>().ok()?),
            None => (s.as_str(), 1),
        };
        if !(1..=MAX_TABLEROS).contains(&board) {
            return None;
        }
        let b = body.as_bytes();
        if b.len() >= 3 && (b[0] == b't' || b[0] == b'b') && (b[1] == b'+' || b[1] == b'-') {
            let col: i32 = body[2..].parse().ok()?;
            return (1..=MAX_COLS).contains(&col).then_some(Hole::Rail {
                board,
                top: b[0] == b't',
                plus: b[1] == b'+',
                col,
            });
        }
        if b.len() >= 2 && (b'a'..=b'j').contains(&b[0]) {
            let col: i32 = body[1..].parse().ok()?;
            return (1..=MAX_COLS).contains(&col).then_some(Hole::Main {
                board,
                row: b[0] - b'a',
                col,
            });
        }
        None
    }
    pub fn name(&self) -> String {
        match *self {
            Hole::Main { board, row, col } => {
                let h = format!("{}{}", (b'a' + row) as char, col);
                if board > 1 {
                    format!("{h}@{board}")
                } else {
                    h
                }
            }
            Hole::Rail {
                board,
                top,
                plus,
                col,
            } => {
                let h = format!(
                    "{}{}{}",
                    if top { 't' } else { 'b' },
                    if plus { '+' } else { '-' },
                    col
                );
                if board > 1 {
                    format!("{h}@{board}")
                } else {
                    h
                }
            }
        }
    }
    pub fn describe(&self) -> String {
        let placa = match self.board() {
            1 => String::new(),
            b => format!("placa {b}, "),
        };
        match *self {
            Hole::Main { row, col, .. } => {
                format!("{placa}fila {}, columna {}", (b'a' + row) as char, col)
            }
            Hole::Rail { top, plus, col, .. } => format!(
                "{placa}riel {} {} ({}), posición {}",
                if plus { "positivo +" } else { "negativo -" },
                if top { "superior" } else { "inferior" },
                if plus { "línea roja" } else { "línea azul" },
                col
            ),
        }
    }
    pub fn strip(&self) -> Strip {
        match *self {
            Hole::Main { board, row, col } => Strip::Column {
                board,
                top: row < 5,
                col,
            },
            Hole::Rail {
                board, top, plus, ..
            } => Strip::Rail { board, top, plus },
        }
    }
    pub fn col(&self) -> i32 {
        match *self {
            Hole::Main { col, .. } | Hole::Rail { col, .. } => col,
        }
    }
    pub fn board(&self) -> u8 {
        match *self {
            Hole::Main { board, .. } | Hole::Rail { board, .. } => board,
        }
    }
    pub fn all(tableros: u8, columnas: i32) -> Vec<Hole> {
        let mut v = Vec::new();
        for board in 1..=tableros.max(1) {
            for col in 1..=columnas.max(1) {
                for row in 0..10u8 {
                    v.push(Hole::Main { board, row, col });
                }
                for (top, plus) in [(true, true), (true, false), (false, false), (false, true)] {
                    v.push(Hole::Rail {
                        board,
                        top,
                        plus,
                        col,
                    });
                }
            }
        }
        v
    }
}

/// Posición de un orificio en unidades de la placa (1 unidad = 2,54 mm).
pub fn hole_xy(h: Hole) -> (f32, f32) {
    let bx = (h.board().saturating_sub(1)) as f32 * BOARD_OFFSET;
    match h {
        Hole::Main { row, col, .. } => (
            bx + col as f32 + 1.0,
            if row < 5 {
                4.0 + row as f32
            } else {
                6.0 + row as f32
            },
        ),
        Hole::Rail { top, plus, col, .. } => (
            bx + col as f32 + 1.0,
            match (top, plus) {
                (true, true) => 1.0,
                (true, false) => 2.0,
                (false, false) => 17.0,
                (false, true) => 18.0,
            },
        ),
    }
}

pub fn board_width(columnas: i32) -> f32 {
    columnas as f32 + 3.0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    And,
    Or,
    Xor,
    Nand,
    Nor,
    Not,
}

impl Gate {
    pub fn eval(self, i: &[bool]) -> bool {
        match self {
            Gate::And => i.iter().all(|x| *x),
            Gate::Or => i.iter().any(|x| *x),
            Gate::Xor => i.iter().filter(|x| **x).count() % 2 == 1,
            Gate::Nand => !i.iter().all(|x| *x),
            Gate::Nor => !i.iter().any(|x| *x),
            Gate::Not => !i[0],
        }
    }
}

pub struct ChipInfo {
    pub modelo: &'static str,
    pub gate: Gate,
    pub nombre: &'static str,
    /// (pines de entrada, pin de salida) de cada compuerta.
    pub gates: &'static [(&'static [u8], u8)],
}

const QUAD: &[(&[u8], u8)] = &[(&[1, 2], 3), (&[4, 5], 6), (&[9, 10], 8), (&[12, 13], 11)];
const NOR4: &[(&[u8], u8)] = &[(&[2, 3], 1), (&[5, 6], 4), (&[8, 9], 10), (&[11, 12], 13)];
const HEX: &[(&[u8], u8)] = &[
    (&[1], 2),
    (&[3], 4),
    (&[5], 6),
    (&[9], 8),
    (&[11], 10),
    (&[13], 12),
];

pub const CHIPS: &[ChipInfo] = &[
    ChipInfo {
        modelo: "74HC08",
        gate: Gate::And,
        nombre: "4 compuertas AND",
        gates: QUAD,
    },
    ChipInfo {
        modelo: "74HC32",
        gate: Gate::Or,
        nombre: "4 compuertas OR",
        gates: QUAD,
    },
    ChipInfo {
        modelo: "74HC86",
        gate: Gate::Xor,
        nombre: "4 compuertas XOR",
        gates: QUAD,
    },
    ChipInfo {
        modelo: "74HC00",
        gate: Gate::Nand,
        nombre: "4 compuertas NAND",
        gates: QUAD,
    },
    ChipInfo {
        modelo: "74HC02",
        gate: Gate::Nor,
        nombre: "4 compuertas NOR",
        gates: NOR4,
    },
    ChipInfo {
        modelo: "74HC04",
        gate: Gate::Not,
        nombre: "6 inversores NOT",
        gates: HEX,
    },
];

/// Acepta la familia 74HC, 74LS y 74HCT (mismo patillaje y misma función a 5 V).
pub fn chip_info(m: &str) -> Option<&'static ChipInfo> {
    let m = m.trim().to_ascii_uppercase().replace([' ', '-'], "");
    CHIPS.iter().find(|c| {
        c.modelo.eq_ignore_ascii_case(&m)
            || match c.modelo {
                "74HC08" => matches!(m.as_str(), "7408" | "74LS08" | "74HCT08"),
                "74HC32" => matches!(m.as_str(), "7432" | "74LS32" | "74HCT32"),
                "74HC86" => matches!(m.as_str(), "7486" | "74LS86" | "74HCT86"),
                "74HC00" => matches!(m.as_str(), "7400" | "74LS00" | "74HCT00"),
                "74HC02" => matches!(m.as_str(), "7402" | "74LS02" | "74HCT02"),
                "74HC04" => matches!(m.as_str(), "7404" | "74LS04" | "74HCT04"),
                _ => false,
            }
    })
}

impl ChipInfo {
    pub fn pin_fn(&self, pin: u8) -> String {
        if pin == 14 {
            return "VCC (+)".into();
        }
        if pin == 7 {
            return "GND (-)".into();
        }
        for (i, (ins, out)) in self.gates.iter().enumerate() {
            if *out == pin {
                return format!("salida de la compuerta {}", i + 1);
            }
            if let Some(k) = ins.iter().position(|p| *p == pin) {
                return format!("entrada {} de la compuerta {}", ["A", "B"][k], i + 1);
            }
        }
        "sin uso".into()
    }
    pub fn pinout(&self) -> String {
        self.gates
            .iter()
            .enumerate()
            .map(|(i, (ins, out))| {
                let ins: Vec<String> = ins.iter().map(|p| p.to_string()).collect();
                let ent = if ins.len() == 1 {
                    format!("entrada {}", ins[0])
                } else {
                    format!("entradas {}", ins.join(" y "))
                };
                format!("compuerta {}: {} → salida {}", i + 1, ent, out)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Orificio del pin `pin` (1..14) de un chip cuyo pin 1 está en la columna `col`, fila f.
pub fn chip_pin_hole(board: u8, col: i32, pin: u8) -> Hole {
    if pin <= 7 {
        Hole::Main {
            board,
            row: 5,
            col: col + pin as i32 - 1,
        }
    } else {
        Hole::Main {
            board,
            row: 4,
            col: col + 14 - pin as i32,
        }
    }
}

/// Orificios del display de 1.2" (10 pines) a partir del pin g (arriba a la izquierda).
/// Orden lógico del vector: a, b, c, d, e, f, g, común 1, común 2, dp.
/// Fila de arriba: g f común a b. Fila de abajo (4 filas más abajo): e d común c dp.
pub fn display_pin_holes(board: u8, row: u8, col: i32) -> Vec<Hole> {
    let h = |r: u8, c: i32| Hole::Main {
        board,
        row: r,
        col: c,
    };
    vec![
        h(row, col + 3),     // a
        h(row, col + 4),     // b
        h(row + 4, col + 3), // c
        h(row + 4, col + 1), // d
        h(row + 4, col),     // e
        h(row, col + 1),     // f
        h(row, col),         // g
        h(row, col + 2),     // común 1
        h(row + 4, col + 2), // común 2
        h(row + 4, col + 4), // dp
    ]
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    #[serde(alias = "bateria", alias = "power", alias = "battery")]
    Fuente,
    #[serde(alias = "resistor")]
    Resistencia,
    Led,
    #[serde(alias = "ic", alias = "compuerta", alias = "integrado")]
    Chip,
    #[serde(alias = "switch", alias = "pulsador", alias = "boton")]
    Interruptor,
    #[serde(alias = "wire", alias = "puente", alias = "jumper")]
    Cable,
    #[serde(
        alias = "display",
        alias = "7seg",
        alias = "segmentos",
        alias = "sevenseg"
    )]
    Display,
    #[serde(alias = "etiqueta", alias = "label", alias = "net", alias = "red")]
    Etiqueta,
}

impl Tipo {
    pub fn prefix(self) -> &'static str {
        match self {
            Tipo::Fuente => "F",
            Tipo::Resistencia => "R",
            Tipo::Led => "D",
            Tipo::Chip => "U",
            Tipo::Interruptor => "S",
            Tipo::Cable => "W",
            Tipo::Display => "DS",
            Tipo::Etiqueta => "L",
        }
    }
    pub fn nombre(self) -> &'static str {
        match self {
            Tipo::Fuente => "Fuente de alimentación",
            Tipo::Resistencia => "Resistencia",
            Tipo::Led => "LED",
            Tipo::Chip => "Chip lógico",
            Tipo::Interruptor => "Interruptor",
            Tipo::Cable => "Cable",
            Tipo::Display => "Display de 7 segmentos",
            Tipo::Etiqueta => "Etiqueta de red",
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn default_columnas() -> i32 {
    COLS
}

fn default_tableros() -> u8 {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Comp {
    #[serde(default)]
    pub id: String,
    pub tipo: Tipo,
    pub pines: Vec<String>,
    /// Ohmios (resistencia) o voltios (fuente).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modelo: Option<String>,
    /// Nombre de la red para las etiquetas (p. ej. "p1", "S2", "GND").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nombre: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub cerrado: bool,
    /// Cable: puntos de quiebre de su recorrido. Fuente: esquina de su caja si se movió.
    /// (unidades de placa)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub puntos: Vec<[f32; 2]>,
    /// Carpeta donde se agrupa, con subcarpetas separadas por "/" (p. ej. "Ecuaciones/F1").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpeta: Option<String>,
}

impl Comp {
    pub fn nuevo(id: &str, tipo: Tipo, pines: &[&str]) -> Comp {
        Comp {
            id: id.into(),
            tipo,
            pines: pines.iter().map(|s| s.to_string()).collect(),
            valor: None,
            color: None,
            modelo: None,
            nombre: None,
            cerrado: false,
            puntos: Vec::new(),
            carpeta: None,
        }
    }
    pub(crate) fn v(mut self, x: f64) -> Self {
        self.valor = Some(x);
        self
    }
    fn c(mut self, s: &str) -> Self {
        self.color = Some(s.into());
        self
    }
    fn m(mut self, s: &str) -> Self {
        self.modelo = Some(s.into());
        self
    }
    #[cfg(test)]
    pub fn n(mut self, s: &str) -> Self {
        self.nombre = Some(s.into());
        self
    }
    fn on(mut self) -> Self {
        self.cerrado = true;
        self
    }
    pub fn ohmios(&self) -> f64 {
        self.valor.unwrap_or(330.0)
    }
    pub fn voltios(&self) -> f64 {
        self.valor.unwrap_or(5.0)
    }
    pub fn color_or(&self, d: &str) -> String {
        self.color
            .clone()
            .unwrap_or_else(|| d.into())
            .trim()
            .to_lowercase()
    }
    pub fn red(&self) -> String {
        self.nombre.clone().unwrap_or_default().trim().to_string()
    }

    /// Orificios que ocupa, en orden de pines (chip: pines 1..14).
    pub fn holes(&self) -> Result<Vec<Hole>, String> {
        if self.tipo == Tipo::Chip {
            let m = self.modelo.clone().unwrap_or_default();
            if chip_info(&m).is_none() {
                let l: Vec<&str> = CHIPS.iter().map(|c| c.modelo).collect();
                return Err(format!(
                    "{}: modelo de chip desconocido '{}' (usa {})",
                    self.id,
                    m,
                    l.join(", ")
                ));
            }
            let p = self
                .pines
                .first()
                .ok_or(format!("{}: falta el orificio del pin 1", self.id))?;
            let (board, col) = match Hole::parse(p) {
                Some(Hole::Main { board, col, .. }) => (board, col),
                _ => {
                    return Err(format!(
                        "{}: '{}' no sirve para el pin 1; usa un orificio de la fila f, p. ej. f10",
                        self.id, p
                    ))
                }
            };
            if col + 6 > MAX_COLS {
                return Err(format!(
                    "{}: el chip no cabe; el pin 1 debe estar en la columna {} o antes",
                    self.id,
                    MAX_COLS - 6
                ));
            }
            return Ok((1..=14).map(|p| chip_pin_hole(board, col, p)).collect());
        }
        if self.tipo == Tipo::Display {
            if self.pines.len() == 1 {
                let (board, row, col) = match Hole::parse(&self.pines[0]) {
                    Some(Hole::Main { board, row, col }) => (board, row, col),
                    _ => {
                        return Err(format!(
                        "{}: '{}' no sirve para el pin g; usa un orificio de la placa, p. ej. c20",
                        self.id, self.pines[0]
                    ))
                    }
                };
                if col + 4 > MAX_COLS {
                    return Err(format!(
                        "{}: el display no cabe; el pin g debe estar en la columna {} o antes",
                        self.id,
                        MAX_COLS - 4
                    ));
                }
                if row + 4 > 9 {
                    return Err(format!(
                        "{}: el display no cabe; el pin g debe estar en la fila {} o antes",
                        self.id,
                        (b'a' + 5) as char
                    ));
                }
                return Ok(display_pin_holes(board, row, col));
            }
            if self.pines.len() != 8 {
                return Err(format!(
                    "{}: un display necesita 1 orificio (pin g del módulo de 10 pines) u 8 pines (modelo antiguo: a, b, c, d, e, f, g y común)",
                    self.id
                ));
            }
            let mut v = Vec::new();
            for p in &self.pines {
                v.push(Hole::parse(p).ok_or(format!(
                    "{}: el orificio '{}' no existe (ejemplos válidos: a1, j30, t+5, b-12, a1@2)",
                    self.id, p
                ))?);
            }
            if v.iter().collect::<HashSet<_>>().len() != v.len() {
                return Err(format!(
                    "{}: los 8 pines del display deben ir en orificios distintos",
                    self.id
                ));
            }
            return Ok(v);
        }
        if self.tipo == Tipo::Etiqueta {
            if self.pines.len() != 1 {
                return Err(format!(
                    "{}: una etiqueta necesita exactamente 1 orificio",
                    self.id
                ));
            }
            return Ok(vec![Hole::parse(&self.pines[0]).ok_or(format!(
                "{}: el orificio '{}' no existe (ejemplos válidos: a1, j30, t+5, b-12, a1@2)",
                self.id, self.pines[0]
            ))?]);
        }
        if self.pines.len() != 2 {
            return Err(format!(
                "{}: un(a) {} necesita exactamente 2 pines",
                self.id,
                self.tipo.nombre()
            ));
        }
        let mut v = Vec::new();
        for p in &self.pines {
            v.push(Hole::parse(p).ok_or(format!(
                "{}: el orificio '{}' no existe (ejemplos válidos: a1, j30, t+5, b-12, a1@2)",
                self.id, p
            ))?);
        }
        if v[0] == v[1] {
            return Err(format!(
                "{}: las dos patas no pueden ir en el mismo orificio",
                self.id
            ));
        }
        Ok(v)
    }

    pub fn pin_label(&self, i: usize) -> String {
        match self.tipo {
            Tipo::Fuente => ["positivo (+)", "negativo (-)"][i.min(1)].into(),
            Tipo::Led => ["ánodo (+, pata larga)", "cátodo (-, pata corta)"][i.min(1)].into(),
            Tipo::Chip => match chip_info(self.modelo.as_deref().unwrap_or("")) {
                Some(info) => format!("pin {}: {}", i + 1, info.pin_fn(i as u8 + 1)),
                None => format!("pin {}", i + 1),
            },
            Tipo::Display => {
                let l: &[&str] = if self.pines.len() != 8 {
                    &[
                        "segmento a",
                        "segmento b",
                        "segmento c",
                        "segmento d",
                        "segmento e",
                        "segmento f",
                        "segmento g",
                        "común 1 (cátodo, -)",
                        "común 2 (cátodo, -)",
                        "punto decimal (dp)",
                    ]
                } else {
                    &[
                        "segmento a",
                        "segmento b",
                        "segmento c",
                        "segmento d",
                        "segmento e",
                        "segmento f",
                        "segmento g",
                        "común (cátodo, -)",
                    ]
                };
                l[i.min(l.len() - 1)].into()
            }
            Tipo::Etiqueta => format!("red «{}»", self.red()),
            Tipo::Cable => format!("extremo {}", i + 1),
            _ => format!("pata {}", i + 1),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Circuito {
    #[serde(default)]
    pub nombre: String,
    #[serde(default = "default_columnas")]
    pub columnas: i32,
    #[serde(default = "default_tableros")]
    pub tableros: u8,
    #[serde(default)]
    pub componentes: Vec<Comp>,
    /// Desplazamiento (x, y) de cada placa respecto a su lugar inicial, en unidades de placa.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub posiciones: Vec<[f32; 2]>,
    /// Carpetas creadas (también las vacías).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carpetas: Vec<String>,
    /// Carpetas ocultas: sus piezas no se dibujan (pero se siguen simulando).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocultas: Vec<String>,
}

impl Default for Circuito {
    fn default() -> Self {
        Circuito {
            nombre: String::new(),
            columnas: COLS,
            tableros: 1,
            componentes: Vec::new(),
            posiciones: Vec::new(),
            carpetas: Vec::new(),
            ocultas: Vec::new(),
        }
    }
}

/// ¿La carpeta `c` es `f` o está dentro de `f`?
pub fn dentro(c: &str, f: &str) -> bool {
    c == f || c.starts_with(&format!("{f}/"))
}

impl Circuito {
    /// Todas las carpetas (creadas o usadas por piezas), con sus carpetas padre, ordenadas.
    pub fn todas_carpetas(&self) -> Vec<String> {
        let mut v: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for f in self
            .carpetas
            .iter()
            .chain(self.componentes.iter().filter_map(|c| c.carpeta.as_ref()))
        {
            let mut acc = String::new();
            for parte in f.split('/').map(str::trim).filter(|p| !p.is_empty()) {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(parte);
                v.insert(acc.clone());
            }
        }
        v.into_iter().collect()
    }
    /// Ids de las piezas de la carpeta `f` y sus subcarpetas.
    pub fn contenido(&self, f: &str) -> Vec<String> {
        self.componentes
            .iter()
            .filter(|c| c.carpeta.as_deref().is_some_and(|x| dentro(x, f)))
            .map(|c| c.id.clone())
            .collect()
    }
    pub fn oculta(&self, c: &Comp) -> bool {
        c.carpeta
            .as_deref()
            .is_some_and(|x| self.ocultas.iter().any(|f| dentro(x, f)))
    }
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if !(8..=MAX_COLS).contains(&self.columnas) {
            errs.push(format!(
                "El circuito debe tener entre 8 y {MAX_COLS} columnas por placa (tiene {}).",
                self.columnas
            ));
        }
        if !(1..=MAX_TABLEROS).contains(&self.tableros) {
            errs.push(format!(
                "El circuito debe tener entre 1 y {MAX_TABLEROS} placas (tiene {}).",
                self.tableros
            ));
        }
        let mut ids = HashSet::new();
        let mut used: HashMap<Hole, String> = HashMap::new();
        for c in &self.componentes {
            if !ids.insert(c.id.clone()) {
                errs.push(format!("id repetido: {}", c.id));
            }
            if c.tipo == Tipo::Resistencia && c.ohmios() <= 0.0 {
                errs.push(format!("{}: el valor (ohmios) debe ser mayor que 0", c.id));
            }
            if c.tipo == Tipo::Fuente && !(0.5..=24.0).contains(&c.voltios()) {
                errs.push(format!("{}: el voltaje debe estar entre 0,5 y 24 V", c.id));
            }
            if c.tipo == Tipo::Etiqueta && c.red().is_empty() {
                errs.push(format!(
                    "{}: la etiqueta necesita un nombre de red (campo \"nombre\").",
                    c.id
                ));
            }
            match c.holes() {
                Err(e) => errs.push(e),
                Ok(hs) => {
                    if c.tipo == Tipo::Chip && hs[0].col() + 6 > self.columnas {
                        errs.push(format!(
                            "{}: el chip no cabe en una placa de {} columnas; mueve el pin 1 a la columna {} o antes.",
                            c.id,
                            self.columnas,
                            self.columnas - 6
                        ));
                        continue;
                    }
                    for h in hs {
                        if h.board() > self.tableros {
                            errs.push(format!(
                                "El orificio {} está en la placa {}, pero el circuito solo tiene {} placa(s).",
                                h.name(),
                                h.board(),
                                self.tableros
                            ));
                        }
                        if h.col() > self.columnas {
                            errs.push(format!(
                                "El orificio {} está en la columna {}, pero la placa solo tiene {} columnas.",
                                h.name(),
                                h.col(),
                                self.columnas
                            ));
                        }
                        if let Some(o) = used.insert(h, c.id.clone()) {
                            errs.push(format!(
                                "El orificio {} ya está ocupado por {} (lo quiere usar {})",
                                h.name(),
                                o,
                                c.id
                            ));
                        }
                    }
                }
            }
        }
        errs
    }
    pub fn next_id(&self, prefix: &str) -> String {
        (1..)
            .map(|n| format!("{prefix}{n}"))
            .find(|id| self.find(id).is_none())
            .unwrap()
    }
    pub fn find(&self, id: &str) -> Option<&Comp> {
        self.componentes.iter().find(|c| c.id == id)
    }
    pub fn completar_ids(&mut self) {
        for i in 0..self.componentes.len() {
            if self.componentes[i].id.trim().is_empty() {
                self.componentes[i].id = self.next_id(self.componentes[i].tipo.prefix());
            }
        }
    }
}

pub const RES_VALS: &[f64] = &[
    100.0, 220.0, 330.0, 470.0, 1000.0, 2200.0, 4700.0, 10000.0, 47000.0, 100000.0,
];
pub const LED_COLORS: &[&str] = &["rojo", "verde", "amarillo", "azul", "blanco"];
pub const WIRE_COLORS: &[&str] = &[
    "rojo", "negro", "azul", "verde", "amarillo", "naranja", "blanco",
];
pub const VOLTS: &[f64] = &[3.3, 4.5, 5.0, 9.0, 12.0];

pub fn fmt_ohm(v: f64) -> String {
    let t = |x: f64| {
        let s = format!("{:.1}", x);
        s.trim_end_matches(".0").replace('.', ",")
    };
    if v >= 1e6 {
        format!("{} MΩ", t(v / 1e6))
    } else if v >= 1e3 {
        format!("{} kΩ", t(v / 1e3))
    } else {
        format!("{} Ω", t(v))
    }
}

// ---------- archivos ----------

pub fn archivo_por_defecto() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("CircuitosIA").join("circuito_actual.json")
}

pub fn cargar(path: &Path) -> Result<Circuito, String> {
    let s = std::fs::read_to_string(path)
        .map_err(|e| format!("No pude leer {}: {e}", path.display()))?;
    parse_text(&s)
}

pub fn guardar(path: &Path, c: &Circuito) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(c).unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Acepta texto con el JSON dentro (por ejemplo la respuesta de un chat de IA con ```json).
pub fn parse_text(s: &str) -> Result<Circuito, String> {
    let a = s.find(['{', '[']);
    let b = s.rfind(['}', ']']);
    let body = match (a, b) {
        (Some(a), Some(b)) if b > a => &s[a..=b],
        _ => return Err("No encontré un circuito en formato JSON en el texto.".into()),
    };
    let mut c = match serde_json::from_str::<Circuito>(body) {
        Ok(c) => c,
        Err(e) => match serde_json::from_str::<Vec<Comp>>(body) {
            Ok(v) => Circuito {
                nombre: String::new(),
                componentes: v,
                ..Default::default()
            },
            Err(_) => return Err(format!("El JSON no es válido: {e}")),
        },
    };
    c.completar_ids();
    Ok(c)
}

// ---------- ejemplos ----------

const PRACTICA5_JSON: &str = include_str!("../assets/practica5.json");
const PRACTICA6_RUMBO_JSON: &str = include_str!("../assets/practica6_rumbo.json");
const PRACTICA6_CELULAR_JSON: &str = include_str!("../assets/practica6_celular.json");

pub fn ejemplos() -> Vec<Circuito> {
    use Tipo::*;
    let k = Comp::nuevo;
    let mut v = vec![
        Circuito {
            nombre: "LED con resistencia".into(),
            componentes: vec![
                k("F1", Fuente, &["t+1", "t-1"]).v(5.0),
                k("W1", Cable, &["t+5", "a5"]).c("rojo"),
                k("R1", Resistencia, &["c5", "c9"]).v(330.0),
                k("D1", Led, &["d9", "d10"]).c("rojo"),
                k("W2", Cable, &["a10", "t-10"]).c("negro"),
            ],
            ..Default::default()
        },
        Circuito {
            nombre: "Compuerta AND con dos interruptores".into(),
            componentes: vec![
                k("F1", Fuente, &["t+1", "t-1"]).v(5.0),
                k("U1", Chip, &["f12"]).m("74HC08"),
                k("W1", Cable, &["t+29", "b+29"]).c("rojo"),
                k("W2", Cable, &["t-30", "b-30"]).c("negro"),
                k("W3", Cable, &["a12", "t+12"]).c("rojo"),
                k("W4", Cable, &["j18", "b-18"]).c("negro"),
                k("W5", Cable, &["b+3", "j3"]).c("rojo"),
                k("S1", Interruptor, &["h3", "h5"]).on(),
                k("R1", Resistencia, &["j5", "b-5"]).v(10000.0),
                k("W6", Cable, &["g5", "g12"]).c("amarillo"),
                k("W7", Cable, &["b+7", "j7"]).c("rojo"),
                k("S2", Interruptor, &["h7", "h9"]).on(),
                k("R2", Resistencia, &["j9", "b-9"]).v(10000.0),
                k("W8", Cable, &["g9", "g13"]).c("azul"),
                k("R3", Resistencia, &["h14", "h20"]).v(330.0),
                k("D1", Led, &["i20", "i21"]).c("verde"),
                k("W9", Cable, &["j21", "b-21"]).c("negro"),
            ],
            ..Default::default()
        },
        Circuito {
            nombre: "Inversor NOT con un interruptor".into(),
            componentes: vec![
                k("F1", Fuente, &["t+1", "t-1"]).v(5.0),
                k("W1", Cable, &["t+3", "b+3"]).c("rojo"),
                k("W2", Cable, &["t-2", "b-2"]).c("negro"),
                k("U1", Chip, &["f10"]).m("74HC04"),
                k("W3", Cable, &["a10", "t+10"]).c("rojo"),
                k("W4", Cable, &["j16", "b-16"]).c("negro"),
                k("W5", Cable, &["b+6", "j6"]).c("rojo"),
                k("S1", Interruptor, &["h6", "h8"]),
                k("R1", Resistencia, &["j8", "b-8"]).v(10000.0),
                k("W6", Cable, &["g8", "g10"]).c("amarillo"),
                k("R2", Resistencia, &["h11", "h20"]).v(330.0),
                k("D1", Led, &["i20", "i21"]).c("rojo"),
                k("W7", Cable, &["j21", "b-21"]).c("negro"),
            ],
            ..Default::default()
        },
    ];
    for json in [PRACTICA5_JSON, PRACTICA6_RUMBO_JSON, PRACTICA6_CELULAR_JSON] {
        if let Ok(p) = serde_json::from_str::<Circuito>(json) {
            v.push(p);
        }
    }
    v
}

// ---------- guía para agentes de IA ----------

pub fn guia() -> String {
    let chips: Vec<String> = CHIPS
        .iter()
        .map(|c| format!("  - {} ({}): {}", c.modelo, c.nombre, c.pinout()))
        .collect();
    let ejemplo = serde_json::to_string_pretty(&ejemplos()[0]).unwrap();
    format!(
        r#"GUÍA DEL FORMATO DE CIRCUITOS - OpenCircuit

Un circuito es un JSON:
{{"nombre": "...", "columnas": 30, "tableros": 1, "componentes": [ ... ]}}
- "columnas": columnas por placa, de 8 a 60 (por defecto 30).
- "tableros": 1 a 3 placas en fila (por defecto 1).
- "carpetas": lista opcional de carpetas vacías; las piezas se agrupan con "carpeta".

LA PROTOBOARD
- Orificios principales: letra de fila + número de columna: "a1", "e12", "j60".
  Filas a-e = mitad superior, f-j = mitad inferior. Los 5 orificios de una columna
  en a-e están conectados entre sí; igual los de f-j. Las dos mitades NO se tocan
  (canal central).
- Para la segunda y tercera placa añade @2 o @3: "a1@2", "t+5@3".
  Sin sufijo se entiende la placa 1.
- Rieles de alimentación (todos sus orificios conectados a lo largo):
  "t+N" riel superior positivo, "t-N" riel superior negativo,
  "b+N" riel inferior positivo, "b-N" riel inferior negativo (N = 1..columnas).
  Los rieles superior e inferior NO están unidos: únelos con cables si hace falta.
- En cada orificio cabe UNA sola pata o punta de cable.
- Las placas NO están unidas por dentro: lleva +5 V y GND de una a otra con cables
  entre sus rieles.

COMPONENTES (campos: id, tipo, pines y, según el tipo, valor / color / modelo / nombre /
cerrado / carpeta)
- {{"id":"F1","tipo":"fuente","valor":5,"pines":["t+1","t-1"]}}
    pines = [+, -]; valor = voltios (5 por defecto).
- {{"id":"R1","tipo":"resistencia","valor":330,"pines":["c5","c9"]}}   valor = ohmios.
- {{"id":"D1","tipo":"led","color":"rojo","pines":["d9","d10"]}}
    pines = [ánodo (+, pata larga), cátodo (-)]. Colores: {leds}.
    Pon siempre una resistencia en serie (220 a 1000 ohmios).
- {{"id":"S1","tipo":"interruptor","cerrado":false,"pines":["h3","h5"]}}
- {{"id":"W1","tipo":"cable","color":"rojo","pines":["t+5","a5"]}}   colores: {wires}.
    "carpeta" es opcional: agrupa la pieza, por ejemplo "Cables/rojo". Las subcarpetas
    se separan con "/". La app las muestra en la pestaña Grupos (enfocar, ocultar, ordenar).
- {{"id":"U1","tipo":"chip","modelo":"74HC08","pines":["f12"]}}
    pines = [orificio del pin 1]. El chip queda sobre el canal central con la muesca
    a la izquierda: pines 1-7 en la fila f, columnas N..N+6; pines 14-8 en la fila e,
    columnas N..N+6 (pin 14 en eN, pin 8 en e(N+6)). N máximo {maxn}.
    Pin 14 = VCC (+) y pin 7 = GND (-): ambos deben ir a la fuente.
    Se aceptan 74HC08/7408/74LS08, 74HC32/7432/74LS32 y 74HC04/7404/74LS04
    (mismo patillaje y misma función a 5 V).
  Modelos:
{chips}
- {{"id":"DS1","tipo":"display","pines":["c20"]}}
    Display de 7 segmentos de 1.2 pulgadas, cátodo común, 10 pines como el módulo real.
    pines = [orificio del pin g, arriba a la izquierda]. La fila de arriba es g, f, común,
    a, b en las columnas N..N+4; la de abajo es e, d, común, c, dp cuatro filas más abajo.
    Los dos comunes van a GND (están unidos por dentro). Cada segmento necesita su
    resistencia (470 ohmios) entre el pin del segmento y la salida que lo controla.
    El punto decimal (dp) puede quedar sin conectar.
    Tambien se acepta el modelo antiguo de 8 pines en una fila: a, b, c, d, e, f, g, común.
- {{"id":"L1","tipo":"etiqueta","nombre":"p1","pines":["g12"]}}
    Etiqueta de red: una sola pata. Dos etiquetas con el MISMO nombre quedan conectadas
    aunque estén en placas distintas. Úsalas para las redes lógicas y para GND.

ETIQUETAS (la forma cómoda de conectar chips en varias placas)
- Pon una etiqueta en la misma columna que el pin, pero en la fila de al lado:
  pin en fN -> etiqueta en gN; pin en eN -> etiqueta en dN. Así la etiqueta toca
  el mismo grupo de 5 orificios que el pin.
- Repite el mismo nombre en todos los pines de esa red (salida y entradas).
- Para una entrada que no se usa, pon una etiqueta "GND" en su fila adyacente.
- Para unir la red GND al negativo, pon una etiqueta "GND" en un orificio del riel b-.

REGLAS PARA QUE FUNCIONE
- Ids únicos: F, R, D, S, W, U, DS, L seguidos de un número.
- Conecta la fuente a los rieles y lleva + y - con cables a donde haga falta.
- Una entrada de chip con un interruptor necesita una resistencia de 10000 ohmios
  hacia el - (pull-down) para no quedar "al aire" cuando el interruptor está abierto.
- No unas nunca + y - directamente (cortocircuito).
- Un display de cátodo común lleva el común a GND y una resistencia por segmento.
  Si tu display tiene dos pines de común, conecta los dos a GND.

EJEMPLO
{ejemplo}
"#,
        leds = LED_COLORS.join(", "),
        wires = WIRE_COLORS.join(", "),
        maxn = MAX_COLS - 6,
        chips = chips.join("\n"),
    )
}
