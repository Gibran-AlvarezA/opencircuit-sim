mod draw;
mod instr;
mod mcp;
mod model;
mod route;
mod sim;
mod svg;

use eframe::egui;
use egui::{Color32, RichText, Sense};
use model::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const AYUDA: &str = "OpenCircuit - simulador de circuitos

Uso:
  opencircuit                         abre la ventana
  opencircuit --archivo <x.json>      usa otro archivo de circuito compartido
  opencircuit --mcp [--archivo ..]    servidor MCP para agentes de IA (stdio)
  opencircuit --guia                  muestra el formato de circuitos
  opencircuit --simular <x.json>      simula un circuito y muestra el resultado
  opencircuit --instrucciones <x.json>  instrucciones de armado en Markdown
  opencircuit --screenshot <x.png>    guarda una captura de la ventana y sale
  opencircuit --exportar <x.svg>      figura vectorial para el reporte (no abre ventana)
  opencircuit --placa A|B|C           exporta solo esa placa (con --exportar)
  opencircuit --ancho <mm>            ancho del SVG (por defecto 420)
  opencircuit --titulo <texto>        título de la figura
  opencircuit --leyenda               añade la leyenda de colores de red
  opencircuit --pestana armado|ia     abre en esa pestaña
  opencircuit --panel grupos          abre el panel de grupos
  opencircuit --grupo <carpeta>       enfoca un grupo al abrir
  opencircuit --solo                  muestra solo el grupo enfocado";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let file = get("--archivo")
        .map(PathBuf::from)
        .unwrap_or_else(archivo_por_defecto);
    if has("--help") || has("--ayuda") {
        println!("{AYUDA}");
        return;
    }
    if has("--mcp") {
        mcp::run(&file);
        return;
    }
    if has("--guia") {
        println!("{}", guia());
        return;
    }
    let leer = |p: String| {
        cargar(Path::new(&p)).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1)
        })
    };
    if let Some(p) = get("--simular") {
        println!("{}", sim::resumen(&leer(p)));
        return;
    }
    if let Some(p) = get("--instrucciones") {
        let c = leer(p);
        let s = sim::simulate(&c);
        let t = sim::tabla(&c);
        println!(
            "{}",
            instr::markdown(&c.nombre, &instr::instrucciones(&c, &s, t.as_ref()))
        );
        return;
    }
    // Exportación vectorial: no abre ventana, así que funciona en CI sin GPU.
    if let Some(destino) = get("--exportar") {
        if destino.to_lowercase().ends_with(".svg") {
            let c = leer(file.to_string_lossy().to_string());
            let s = sim::simulate(&c);
            let placa = get("--placa").and_then(|p| match p.to_uppercase().as_str() {
                "A" | "1" => Some(1u8),
                "B" | "2" => Some(2u8),
                "C" | "3" => Some(3u8),
                otro => otro.parse::<u8>().ok().filter(|n| (1..=3).contains(n)),
            });
            let cfg = svg::Cfg {
                placa,
                ancho_mm: get("--ancho")
                    .and_then(|x| x.parse::<f32>().ok())
                    .unwrap_or(420.0),
                titulo: get("--titulo"),
                leyenda: has("--leyenda"),
            };
            let out = svg::exportar(&c, &s, &cfg);
            if let Some(dir) = Path::new(&destino).parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            match std::fs::write(&destino, out) {
                Ok(()) => println!("SVG: {destino}"),
                Err(e) => {
                    eprintln!("No pude escribir {destino}: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }
    }
    let shot = get("--screenshot").map(PathBuf::from);
    let tab = match get("--pestana").as_deref() {
        Some("armado") => Tab::Armado,
        Some("ia") => Tab::Ia,
        _ => Tab::Protoboard,
    };
    let lado = if get("--panel").as_deref() == Some("grupos") {
        LadoPanel::Grupos
    } else {
        LadoPanel::Piezas
    };
    let grupo = get("--grupo");
    let solo = has("--solo");

    let mut vp = egui::ViewportBuilder::default()
        .with_title("OpenCircuit - simulador de circuitos")
        .with_inner_size([1440.0, 880.0])
        .with_min_inner_size([960.0, 620.0]);
    if shot.is_some() {
        vp = vp.with_active(false);
        #[cfg(windows)]
        {
            vp = vp.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut opts = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if shot.is_some() {
        opts.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    let r = eframe::run_native(
        "opencircuit",
        opts,
        Box::new(move |cc| {
            tema(&cc.egui_ctx);
            let mut app = App::new(file, shot);
            app.tab = tab;
            app.lado = lado;
            if let Some(g) = grupo {
                app.grupo = Some(g);
            }
            app.solo_grupo = solo;
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = r {
        eprintln!("Error al abrir la ventana: {e}");
        std::process::exit(1);
    }
}

const MUTED: Color32 = Color32::from_rgb(150, 160, 174);
const ACCENT: Color32 = Color32::from_rgb(110, 168, 254);
const SUCCESS: Color32 = Color32::from_rgb(74, 222, 128);
const WARN: Color32 = Color32::from_rgb(251, 191, 36);
const DANGER: Color32 = Color32::from_rgb(248, 113, 113);
const CARD: Color32 = Color32::from_rgb(31, 38, 47);
const PANEL: Color32 = Color32::from_rgb(22, 27, 33);

fn tema(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(15, 19, 24);
    v.faint_bg_color = Color32::from_rgb(36, 43, 53);
    v.code_bg_color = Color32::from_rgb(15, 19, 24);
    v.override_text_color = Some(Color32::from_rgb(226, 232, 240));
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(110, 168, 254, 70);
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(45, 54, 66));
    v.widgets.inactive.bg_fill = Color32::from_rgb(38, 46, 57);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 46, 57);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(52, 62, 76));
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(90, 102, 118));
    v.widgets.hovered.bg_fill = Color32::from_rgb(48, 58, 72);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(48, 58, 72);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(140, 154, 172));
    v.widgets.active.bg_fill = Color32::from_rgb(56, 70, 88);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(56, 70, 88);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    v.widgets.active.corner_radius = egui::CornerRadius::same(6);
    v.widgets.open.corner_radius = egui::CornerRadius::same(6);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 7.0);
        s.spacing.button_padding = egui::vec2(9.0, 5.0);
        s.spacing.interact_size.y = 24.0;
    });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .strong()
            .color(MUTED),
    );
    ui.add_space(1.0);
}

fn dot(ui: &mut egui::Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(r.center(), 4.0, color);
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn padre_colapsado(f: &str, colapsadas: &HashSet<String>) -> bool {
    let mut p = f;
    while let Some(i) = p.rfind('/') {
        p = &p[..i];
        if colapsadas.contains(p) {
            return true;
        }
    }
    false
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Protoboard,
    Armado,
    Ia,
}

#[derive(PartialEq, Clone, Copy)]
enum LadoPanel {
    Piezas,
    Grupos,
}

#[derive(PartialEq, Clone, Copy)]
enum Tool {
    Select,
    Delete,
    Place(Tipo),
}

enum Drag {
    Caja(egui::Pos2),
    /// Fuente que se arrastra (id, distancia del ratón a la esquina de su caja).
    Fuente(String, egui::Vec2),
    Pan,
    Board(u8),
    Punto(String, usize),
}

enum Act {
    Toggle(String),
    Select(String),
    Deselect,
    Delete(String),
    Cancel,
    Place(Tipo, Hole),
}

struct App {
    circ: Circuito,
    sim: sim::Sim,
    tabla: Option<sim::Tabla>,
    pasos: Vec<instr::Paso>,
    undo: Vec<Circuito>,
    redo: Vec<Circuito>,
    tab: Tab,
    lado: LadoPanel,
    /// Carpeta enfocada; `Some("")` es «Sin grupo».
    grupo: Option<String>,
    solo_grupo: bool,
    colapsadas: HashSet<String>,
    nueva_carpeta: String,
    mover_a: String,
    buscar_cable: String,
    sep_cables: f32,
    ultimo_rect: Option<egui::Rect>,
    tool: Tool,
    chip_model: String,
    res_val: f64,
    led_color: String,
    wire_color: String,
    label_name: String,
    zoom: f32,
    pan: egui::Vec2,
    drag: Option<Drag>,
    /// Selección múltiple (Ctrl/Shift + clic, o Shift + arrastrar una caja).
    sel: HashSet<String>,
    pending: Option<Hole>,
    selected: Option<String>,
    paso: usize,
    file: PathBuf,
    mtime: Option<SystemTime>,
    next_poll: f64,
    status: String,
    pegar: String,
    pegar_msg: String,
    shot: Option<PathBuf>,
    frames: u32,
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn layer(t: Tipo) -> u8 {
    match t {
        Tipo::Chip => 0,
        Tipo::Display => 1,
        Tipo::Resistencia => 2,
        Tipo::Led => 3,
        Tipo::Interruptor => 4,
        Tipo::Cable => 5,
        Tipo::Etiqueta => 6,
        Tipo::Fuente => 7,
    }
}

impl App {
    fn new(file: PathBuf, shot: Option<PathBuf>) -> App {
        let loaded = cargar(&file).ok();
        let fresh = loaded.is_none();
        let mut app = App {
            circ: loaded.unwrap_or_else(|| ejemplos()[1].clone()),
            sim: Default::default(),
            tabla: None,
            pasos: vec![],
            undo: vec![],
            redo: vec![],
            tab: Tab::Protoboard,
            lado: LadoPanel::Piezas,
            grupo: None,
            solo_grupo: false,
            colapsadas: HashSet::new(),
            nueva_carpeta: String::new(),
            mover_a: String::new(),
            buscar_cable: String::new(),
            sep_cables: 4.0,
            ultimo_rect: None,
            tool: Tool::Select,
            chip_model: "74HC08".into(),
            res_val: 330.0,
            led_color: "rojo".into(),
            wire_color: "rojo".into(),
            label_name: "X".into(),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            drag: None,
            sel: HashSet::new(),
            pending: None,
            selected: None,
            paso: 0,
            mtime: mtime(&file),
            file,
            next_poll: 0.0,
            status: "Listo. Pasa el ratón por la protoboard para medir voltajes; haz clic en un interruptor para cambiarlo.".into(),
            pegar: String::new(),
            pegar_msg: String::new(),
            shot,
            frames: 0,
        };
        app.recompute();
        App::ordenar_cables(&mut app.circ);
        if fresh {
            app.save();
        }
        app
    }

    /// Deja rectos los cables que aún no tienen trazado al abrir un circuito (de ejemplo o de una IA).
    /// Los que la persona ya acomodó a mano (con quiebres) no se tocan.
    fn ordenar_cables(c: &mut Circuito) {
        let ids: Vec<String> = c
            .componentes
            .iter()
            .filter(|x| x.tipo == Tipo::Cable && x.puntos.is_empty())
            .map(|x| x.id.clone())
            .collect();
        if !ids.is_empty() {
            route::ordenar(c, &ids);
        }
    }

    fn recompute(&mut self) {
        self.sim = sim::simulate(&self.circ);
        let chips = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Chip)
            .count();
        let switches = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Interruptor)
            .count();
        // La tabla de 32 casos con 12 chips tarda; en circuitos grandes se calcula a petición.
        self.tabla = if chips <= 4 && switches <= 4 {
            sim::tabla(&self.circ)
        } else {
            None
        };
        self.pasos = instr::instrucciones(&self.circ, &self.sim, self.tabla.as_ref());
        self.paso = self.paso.min(self.pasos.len().saturating_sub(1));
        if let Some(id) = &self.selected {
            if self.circ.find(id).is_none() {
                self.selected = None;
            }
        }
        let circ = &self.circ;
        self.sel.retain(|id| circ.find(id).is_some());
    }

    fn save(&mut self) {
        if self.shot.is_some() {
            return; // una captura de comprobación nunca toca el circuito de la persona
        }
        match guardar(&self.file, &self.circ) {
            Ok(()) => self.mtime = mtime(&self.file),
            Err(e) => self.status = format!("No pude guardar el circuito: {e}"),
        }
    }

    fn modify(&mut self, f: impl FnOnce(&mut Circuito)) {
        self.undo.push(self.circ.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        f(&mut self.circ);
        self.recompute();
        self.save();
    }

    fn set_circuit(&mut self, c: Circuito, msg: String) {
        self.modify(|k| {
            *k = c;
            App::ordenar_cables(k);
        });
        self.selected = None;
        self.sel.clear();
        self.pending = None;
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
        self.paso = 0;
        self.grupo = None;
        self.solo_grupo = false;
        self.status = msg;
    }

    fn en_grupo(c: &Comp, f: &str) -> bool {
        if f.is_empty() {
            c.carpeta.is_none()
        } else {
            c.carpeta.as_deref().is_some_and(|x| dentro(x, f))
        }
    }

    fn ids_de(&self, f: &str) -> Vec<String> {
        if f.is_empty() {
            self.circ
                .componentes
                .iter()
                .filter(|c| c.carpeta.is_none())
                .map(|c| c.id.clone())
                .collect()
        } else {
            self.circ.contenido(f)
        }
    }

    fn nombre_grupo(&self) -> String {
        self.grupo
            .as_deref()
            .map(|f| {
                if f.is_empty() {
                    "Sin grupo".to_string()
                } else {
                    f.to_string()
                }
            })
            .unwrap_or_default()
    }

    fn seleccionados(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sel.iter().cloned().collect();
        if let Some(id) = &self.selected {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    fn mover_a_carpeta(&mut self, ids: &[String], folder: Option<String>) {
        if ids.is_empty() {
            return;
        }
        let set: HashSet<String> = ids.iter().cloned().collect();
        let n = set.len();
        let f = folder.clone();
        self.modify(|c| {
            for x in c.componentes.iter_mut().filter(|x| set.contains(&x.id)) {
                x.carpeta = f.clone();
            }
            if let Some(f) = &f {
                if !c.carpetas.iter().any(|x| x == f) {
                    c.carpetas.push(f.clone());
                }
            }
        });
        self.status = format!(
            "{n} pieza(s) movidas a {}.",
            folder.as_deref().unwrap_or("Sin grupo")
        );
    }

    fn crear_carpeta(&mut self, nombre: &str) {
        let f = nombre.trim().trim_matches('/').replace("//", "/");
        if f.is_empty() {
            self.status = "Escribe un nombre de carpeta (por ejemplo Cables/rojo).".into();
            return;
        }
        if self.circ.todas_carpetas().iter().any(|x| x == &f) {
            self.status = format!("La carpeta «{f}» ya existe.");
            return;
        }
        let f2 = f.clone();
        self.modify(|c| c.carpetas.push(f2));
        self.nueva_carpeta.clear();
        self.status = format!("Carpeta «{f}» creada.");
    }

    fn agrupar_cables_por_color(&mut self) {
        let mut n = 0;
        self.modify(|c| {
            for i in 0..c.componentes.len() {
                if c.componentes[i].tipo != Tipo::Cable || c.componentes[i].carpeta.is_some() {
                    continue;
                }
                let f = format!("Cables/{}", c.componentes[i].color_or("rojo"));
                c.componentes[i].carpeta = Some(f.clone());
                if !c.carpetas.iter().any(|x| x == &f) {
                    c.carpetas.push(f);
                }
                n += 1;
            }
        });
        self.status = if n == 0 {
            "Todos los cables ya tenían grupo.".into()
        } else {
            format!("{n} cable(s) agrupados por color en Cables/<color>.")
        };
    }

    fn toggle_oculta(&mut self, f: &str) {
        let f = f.to_string();
        let f2 = f.clone();
        self.modify(|c| {
            if let Some(i) = c.ocultas.iter().position(|x| x == &f2) {
                c.ocultas.remove(i);
            } else {
                c.ocultas.push(f2);
            }
        });
        let oculta = self.circ.ocultas.iter().any(|x| x == &f);
        self.status = if oculta {
            format!("Carpeta «{f}» oculta. La simulación no cambia.")
        } else {
            format!("Carpeta «{f}» visible.")
        };
    }

    fn mostrar_todo(&mut self) {
        self.modify(|c| c.ocultas.clear());
        self.grupo = None;
        self.solo_grupo = false;
        self.status = "Todas las carpetas visibles; enfoque quitado.".into();
    }

    fn borrar_carpeta_vacia(&mut self, f: &str) {
        let tiene_hijos = self
            .circ
            .todas_carpetas()
            .iter()
            .any(|x| x.starts_with(&format!("{f}/")));
        if tiene_hijos || !self.circ.contenido(f).is_empty() {
            self.status = format!("«{f}» no está vacía; mueve su contenido antes de borrarla.");
            return;
        }
        let f2 = f.to_string();
        self.modify(|c| {
            c.carpetas.retain(|x| x != &f2);
            c.ocultas.retain(|x| x != &f2);
        });
        if self.grupo.as_deref() == Some(f) {
            self.grupo = None;
        }
        self.status = format!("Carpeta vacía «{f}» borrada.");
    }

    fn enfocar(&mut self, f: &str) {
        if self.grupo.as_deref() == Some(f) {
            self.grupo = None;
            self.solo_grupo = false;
            self.status = "Enfoque quitado.".into();
        } else {
            self.grupo = Some(f.to_string());
            self.status = format!(
                "Enfocando «{}». El resto queda atenuado.",
                if f.is_empty() { "Sin grupo" } else { f }
            );
        }
    }

    /// Centra la vista en las piezas indicadas usando sus orificios (coordenadas de pantalla).
    fn centrar_ids(&mut self, ids: &[String]) {
        let Some(rect) = self.ultimo_rect else { return };
        let v = self.view_for(rect);
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for id in ids {
            let Some(c) = self.circ.find(id) else {
                continue;
            };
            let Ok(hs) = c.holes() else { continue };
            for h in hs {
                let p = v.h(h);
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
            }
        }
        if min.x == f32::MAX {
            return;
        }
        let centro = egui::pos2((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
        self.pan += rect.center() - centro;
    }

    fn centrar_grupo(&mut self, f: &str) {
        let ids = self.ids_de(f);
        self.centrar_ids(&ids);
        self.status = format!(
            "Vista centrada en «{}».",
            if f.is_empty() { "Sin grupo" } else { f }
        );
    }

    /// Rutea cables ordenados por carpeta y color; si `solo` tiene carpeta, solo esos.
    fn ordenar_grupos(&mut self, solo: Option<&str>) {
        let mut cables: Vec<(String, String, String)> = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .filter(|c| solo.is_none_or(|f| Self::en_grupo(c, f)))
            .map(|c| {
                (
                    c.carpeta.clone().unwrap_or_default(),
                    c.color_or("rojo"),
                    c.id.clone(),
                )
            })
            .collect();
        cables.sort();
        let ids: Vec<String> = cables.into_iter().map(|(_, _, id)| id).collect();
        if ids.is_empty() {
            self.status = "No hay cables que ordenar en ese grupo.".into();
            return;
        }
        let n = ids.len();
        let sep = if solo.is_some() { 8.0 } else { self.sep_cables };
        let mut ok = 0;
        self.modify(|c| ok = route::ordenar_con(c, &ids, sep));
        let donde = solo
            .map(|f| format!(" en «{}»", if f.is_empty() { "Sin grupo" } else { f }))
            .unwrap_or_default();
        self.status = format!(
            "{ok}/{n} cables ordenados{donde} · {} sin ruta (quedaron curvos).",
            n - ok
        );
    }

    fn poll_file(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if now < self.next_poll {
            return;
        }
        self.next_poll = now + 1.0;
        let m = mtime(&self.file);
        if m.is_none() || m == self.mtime {
            return;
        }
        self.mtime = m;
        match cargar(&self.file) {
            Ok(c) if c != self.circ => {
                self.undo.push(std::mem::replace(&mut self.circ, c));
                self.redo.clear();
                Self::ordenar_cables(&mut self.circ);
                self.recompute();
                self.status =
                    "El circuito se actualizó desde el archivo compartido (agente de IA).".into();
            }
            Ok(_) => {}
            Err(e) => self.status = format!("El archivo compartido tiene un error: {e}"),
        }
    }

    fn occupied(&self, h: Hole) -> Option<String> {
        self.circ
            .componentes
            .iter()
            .find(|c| c.holes().is_ok_and(|hs| hs.contains(&h)))
            .map(|c| c.id.clone())
    }

    fn toggle(&mut self, id: &str) {
        let id = id.to_string();
        self.modify(|c| {
            if let Some(x) = c.componentes.iter_mut().find(|x| x.id == id) {
                x.cerrado = !x.cerrado;
            }
        });
    }

    fn delete(&mut self, id: &str) {
        let id = id.to_string();
        self.modify(|c| c.componentes.retain(|x| x.id != id));
        self.status = format!("Eliminado {id}.");
    }

    fn try_add(&mut self, c: Comp) {
        let antes = self.circ.validate();
        let mut test = self.circ.clone();
        test.componentes.push(c.clone());
        let errs: Vec<String> = test
            .validate()
            .into_iter()
            .filter(|e| !antes.contains(e))
            .collect();
        if errs.is_empty() {
            let id = c.id.clone();
            self.modify(|k| k.componentes.push(c));
            self.status = format!("Añadido {id}. Clic derecho sobre una pieza para quitarla.");
            self.selected = Some(id);
        } else {
            self.status = errs.join(" · ");
        }
    }

    fn place(&mut self, t: Tipo, h: Hole) {
        let mut c = Comp::nuevo(&self.circ.next_id(t.prefix()), t, &[]);
        match t {
            Tipo::Resistencia => c.valor = Some(self.res_val),
            Tipo::Fuente => c.valor = Some(5.0),
            Tipo::Led => c.color = Some(self.led_color.clone()),
            Tipo::Cable => c.color = Some(self.wire_color.clone()),
            Tipo::Chip => c.modelo = Some(self.chip_model.clone()),
            Tipo::Etiqueta => c.nombre = Some(self.label_name.trim().to_string()),
            Tipo::Display | Tipo::Interruptor => {}
        }
        if t == Tipo::Etiqueta {
            if c.red().is_empty() {
                self.status = "Escribe un nombre de red antes de colocar la etiqueta.".into();
                return;
            }
            c.pines = vec![h.name()];
            self.try_add(c);
            return;
        }
        if t == Tipo::Display {
            let (board, row, col) = match h {
                Hole::Main { board, row, col } => (board, row, col),
                _ => {
                    self.status =
                        "El display va sobre los orificios principales, no en los rieles.".into();
                    return;
                }
            };
            if col + 7 > self.circ.columnas {
                self.status = format!(
                    "El display no cabe: el terminal a debe estar en la columna {} o antes.",
                    self.circ.columnas - 7
                );
                return;
            }
            c.pines = (0..8)
                .map(|i| {
                    Hole::Main {
                        board,
                        row,
                        col: col + i,
                    }
                    .name()
                })
                .collect();
            self.try_add(c);
            return;
        }
        if t == Tipo::Chip {
            let (board, col) = match h {
                Hole::Main { board, col, .. } => (board, col),
                _ => {
                    self.status =
                        "El chip va en las filas centrales: haz clic en la fila e o f.".into();
                    return;
                }
            };
            let col = col.min(self.circ.columnas - 6);
            c.pines = vec![Hole::Main { board, row: 5, col }.name()];
            self.try_add(c);
            return;
        }
        match self.pending {
            None => {
                if let Some(o) = self.occupied(h) {
                    self.status = format!("El orificio {} ya lo usa {}.", h.name(), o);
                } else {
                    self.pending = Some(h);
                }
            }
            Some(a) => {
                self.pending = None;
                if a != h {
                    c.pines = vec![a.name(), h.name()];
                    let (id, cable) = (c.id.clone(), c.tipo == Tipo::Cable);
                    self.try_add(c);
                    if cable && self.circ.find(&id).is_some() {
                        route::ordenar(&mut self.circ, &[id]); // recto y ordenado por defecto
                        self.save();
                    }
                }
            }
        }
    }

    fn apply(&mut self, a: Act) {
        match a {
            Act::Select(id) => {
                if self
                    .circ
                    .find(&id)
                    .is_some_and(|c| c.tipo == Tipo::Interruptor)
                {
                    self.toggle(&id);
                }
                self.sel.clear();
                self.selected = Some(id);
            }
            Act::Deselect => {
                self.selected = None;
                self.sel.clear();
            }
            Act::Toggle(id) => {
                if let Some(p) = self.selected.take() {
                    self.sel.insert(p);
                }
                if !self.sel.remove(&id) {
                    self.sel.insert(id);
                }
                if self.sel.len() == 1 {
                    self.selected = self.sel.drain().next();
                }
            }
            Act::Delete(id) => self.delete(&id),
            Act::Cancel => self.pending = None,
            Act::Place(t, h) => self.place(t, h),
        }
    }

    fn hint(&self) -> String {
        match (self.tool, self.pending) {
            (Tool::Select, _) => "Clic en una pieza para ver sus datos; clic en un interruptor para abrirlo o cerrarlo; clic derecho para quitarla.".into(),
            (Tool::Delete, _) => "Haz clic sobre la pieza que quieras quitar.".into(),
            (Tool::Place(Tipo::Chip), _) => "Haz clic en la columna donde irá el pin 1 (fila e o f). La muesca queda a la izquierda.".into(),
            (Tool::Place(Tipo::Display), _) => "Haz clic en el orificio del terminal a; los 8 terminales se colocan en la misma fila hacia la derecha.".into(),
            (Tool::Place(Tipo::Etiqueta), _) => "Haz clic en el orificio donde va la etiqueta. Se conecta con las etiquetas del mismo nombre.".into(),
            (Tool::Place(Tipo::Fuente), None) => "Clic en el orificio para el + (normalmente el riel rojo).".into(),
            (Tool::Place(Tipo::Fuente), Some(_)) => "Ahora clic en el orificio para el - (riel azul). Esc cancela.".into(),
            (Tool::Place(Tipo::Led), None) => "Clic en el orificio de la pata larga (ánodo, +).".into(),
            (Tool::Place(Tipo::Led), Some(_)) => "Ahora clic en el orificio de la pata corta (cátodo, -). Esc cancela.".into(),
            (Tool::Place(t), None) => format!("{}: clic en el orificio de la primera pata.", t.nombre()),
            (Tool::Place(t), Some(_)) => format!("{}: ahora clic en el orificio de la segunda pata. Esc cancela.", t.nombre()),
        }
    }

    fn view_for(&self, rect: egui::Rect) -> draw::View {
        let mut v = draw::View::fit(
            rect,
            self.circ.tableros,
            self.circ.columnas,
            &self.circ.posiciones,
        );
        let c = rect.center();
        v.o = c + (v.o - c) * self.zoom + self.pan;
        v.s *= self.zoom;
        for (i, d) in self.circ.posiciones.iter().take(16).enumerate() {
            v.off[i] = egui::vec2(d[0], d[1]);
        }
        v
    }

    /// Qué arrastra el ratón: un punto de un cable, un cable (crea un quiebre), una placa o la vista.
    fn drag_target(&mut self, view: &draw::View, m: egui::Pos2) -> Drag {
        let s = view.s;
        if self.tool != Tool::Select {
            return Drag::Pan;
        }
        let (mx, my) = view.inv(m);
        let fuentes: Vec<&Comp> = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Fuente)
            .collect();
        if let Some((i, f)) = fuentes.iter().enumerate().find(|(i, f)| {
            !self.circ.oculta(f) && draw::fuente_box(f, *i).contains(egui::pos2(mx, my))
        }) {
            let b = draw::fuente_box(f, i);
            self.undo.push(self.circ.clone());
            return Drag::Fuente(f.id.clone(), egui::vec2(mx - b.left(), my - b.top()));
        }
        if let Some(c) = self
            .selected
            .as_ref()
            .and_then(|id| self.circ.find(id))
            .filter(|c| c.tipo == Tipo::Cable)
        {
            if let Some(i) = c
                .puntos
                .iter()
                .position(|q| view.p(q[0], q[1]).distance(m) < 0.45 * s)
            {
                self.undo.push(self.circ.clone());
                return Drag::Punto(c.id.clone(), i);
            }
        }
        let cable = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable && !self.circ.oculta(c))
            .find_map(|c| {
                let hs = c.holes().ok()?;
                let pts = draw::cable_pts(view, c, &hs);
                (draw::poly_dist(m, &pts) < 0.35 * s).then(|| {
                    // índice del tramo más cercano (solo cuenta si el cable ya tiene quiebres)
                    let k = if c.puntos.is_empty() {
                        0
                    } else {
                        (0..pts.len() - 1)
                            .min_by(|&a, &b| {
                                draw::poly_dist(m, &pts[a..a + 2])
                                    .total_cmp(&draw::poly_dist(m, &pts[b..b + 2]))
                            })
                            .unwrap()
                    };
                    (c.id.clone(), k)
                })
            });
        if let Some((id, k)) = cable {
            let (x, y) = view.inv(m);
            self.modify(|c| {
                if let Some(w) = c.componentes.iter_mut().find(|w| w.id == id) {
                    w.puntos.insert(k, [x, y]);
                }
            });
            self.selected = Some(id.clone());
            return Drag::Punto(id, k);
        }
        let on_comp = self
            .circ
            .componentes
            .iter()
            .filter(|c| !self.circ.oculta(c))
            .any(|c| {
                c.holes().is_ok_and(|hs| {
                    draw::hit(view, c, &hs, m, 0).is_some() && c.tipo != Tipo::Fuente
                })
            });
        if !on_comp {
            if let Some(b) = (1..=self.circ.tableros.max(1))
                .rev()
                .find(|&b| view.board_rect(b).contains(m))
            {
                self.undo.push(self.circ.clone());
                return Drag::Board(b);
            }
        }
        Drag::Pan
    }

    fn board(&mut self, ui: &mut egui::Ui, assembly: bool) {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        self.ultimo_rect = Some(rect);
        draw::mat(&painter, rect);
        let mut view = if assembly {
            // La guía describe las placas en fila: la vista previa las muestra así, no donde se arrastraron.
            draw::View::fit(rect, self.circ.tableros, self.circ.columnas, &[])
        } else {
            self.view_for(rect)
        };
        if let Some(pos) = resp
            .hover_pos()
            .filter(|_| !assembly && self.shot.is_none())
        {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let world = view.inv(pos);
                let z = (self.zoom * (scroll * 0.0015).exp()).clamp(0.5, 12.0);
                if (z - self.zoom).abs() > 1e-4 {
                    self.zoom = z;
                    let nv = self.view_for(rect);
                    self.pan += pos - nv.p(world.0, world.1);
                    view = self.view_for(rect);
                }
            }
        }
        if resp.drag_started() {
            let m = ui.input(|i| i.pointer.press_origin()).or(resp.hover_pos());
            self.drag = Some(match (assembly, m) {
                (false, Some(m))
                    if self.tool == Tool::Select && ui.input(|i| i.modifiers.shift) =>
                {
                    Drag::Caja(m)
                }
                (false, Some(m)) => self.drag_target(&view, m),
                _ => Drag::Pan,
            });
        }
        if resp.dragged() {
            let d = resp.drag_delta();
            match &self.drag {
                Some(Drag::Board(b)) => {
                    let i = *b as usize - 1;
                    if self.circ.posiciones.len() <= i {
                        self.circ.posiciones.resize(i + 1, [0.0, 0.0]);
                    }
                    self.circ.posiciones[i][0] += d.x / view.s;
                    self.circ.posiciones[i][1] += d.y / view.s;
                    // los quiebres de cables que viven en esta placa viajan con ella
                    for w in self
                        .circ
                        .componentes
                        .iter_mut()
                        .filter(|w| w.tipo == Tipo::Cable)
                    {
                        if w.holes().is_ok_and(|hs| hs.iter().all(|h| h.board() == *b)) {
                            for q in &mut w.puntos {
                                q[0] += d.x / view.s;
                                q[1] += d.y / view.s;
                            }
                        }
                    }
                }
                Some(Drag::Fuente(id, grab)) => {
                    if let Some(m) = resp.hover_pos().or(ui.input(|i| i.pointer.latest_pos())) {
                        let (x, y) = view.inv(m);
                        if let Some(f) = self.circ.componentes.iter_mut().find(|f| &f.id == id) {
                            f.puntos = vec![[x - grab.x, y - grab.y]];
                        }
                    }
                }
                Some(Drag::Punto(id, i)) => {
                    if let Some(m) = resp.hover_pos().or(ui.input(|i| i.pointer.latest_pos())) {
                        let (x, y) = view.inv(m);
                        let snap = |v: f32| (v * 2.0).round() / 2.0; // se alinea a medio orificio
                        if let Some(w) = self.circ.componentes.iter_mut().find(|w| &w.id == id) {
                            if let Some(q) = w.puntos.get_mut(*i) {
                                *q = [snap(x), snap(y)];
                            }
                        }
                    }
                }
                _ => self.pan += d,
            }
            view = self.view_for(rect);
        }
        if let (Some(Drag::Caja(a)), Some(b)) = (
            &self.drag,
            resp.hover_pos().or(ui.input(|i| i.pointer.latest_pos())),
        ) {
            let r = egui::Rect::from_two_pos(*a, b);
            if resp.drag_stopped() {
                self.selected = None;
                self.sel = self
                    .circ
                    .componentes
                    .iter()
                    .filter(|c| !self.circ.oculta(c))
                    .filter(|c| {
                        c.holes()
                            .is_ok_and(|hs| hs.iter().all(|h| r.contains(view.h(*h))))
                    })
                    .map(|c| c.id.clone())
                    .collect();
                if self.sel.len() == 1 {
                    self.selected = self.sel.drain().next();
                }
            } else {
                painter.rect_filled(r, 2.0, Color32::from_rgba_unmultiplied(255, 215, 0, 30));
                painter.rect_stroke(
                    r,
                    2.0,
                    egui::Stroke::new(1.5_f32, Color32::YELLOW),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if resp.drag_stopped() {
            if matches!(
                self.drag,
                Some(Drag::Board(_) | Drag::Punto(..) | Drag::Fuente(..))
            ) {
                self.save();
            }
            self.drag = None;
        }
        let quitar_punto = resp
            .double_clicked()
            .then(|| resp.hover_pos())
            .flatten()
            .and_then(|m| {
                let c = self.circ.find(self.selected.as_deref()?)?;
                let i = c
                    .puntos
                    .iter()
                    .position(|q| view.p(q[0], q[1]).distance(m) < 0.45 * view.s)?;
                Some((c.id.clone(), i))
            });
        if let Some((id, i)) = quitar_punto {
            self.modify(|c| {
                if let Some(w) = c.componentes.iter_mut().find(|w| w.id == id) {
                    w.puntos.remove(i);
                }
            });
        } else if resp.double_clicked() {
            self.zoom = 1.0;
            self.pan = egui::Vec2::ZERO;
            view = self.view_for(rect);
        }
        let s = view.s;
        let t = ui.input(|i| i.time);
        let hover = resp.hover_pos();
        let hh = hover.and_then(|m| view.nearest_hole(m));

        let (visible, current): (Option<HashSet<String>>, HashSet<String>) = if assembly {
            let vis = self
                .pasos
                .iter()
                .take(self.paso + 1)
                .flat_map(|p| p.ids.iter().cloned())
                .collect();
            let cur = self
                .pasos
                .get(self.paso)
                .map(|p| p.ids.iter().cloned().collect())
                .unwrap_or_default();
            (Some(vis), cur)
        } else {
            (None, HashSet::new())
        };
        let live = visible
            .as_ref()
            .is_none_or(|v| self.circ.componentes.iter().all(|c| v.contains(&c.id)));
        let empty = sim::Sim::default();
        let simref = if live { &self.sim } else { &empty };

        let mut hl = HashSet::new();
        if let Some(h) = hh {
            let net = if live { self.sim.net_of(h) } else { None };
            for x in Hole::all(self.circ.tableros, self.circ.columnas) {
                if match net {
                    Some(n) => self.sim.net_of(x) == Some(n),
                    None => x.strip() == h.strip(),
                } {
                    hl.insert(x);
                }
            }
        }
        draw::board(&painter, &view, &hl);

        let mut fidx: HashMap<String, usize> = HashMap::new();
        for c in self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Fuente)
        {
            let n = fidx.len();
            fidx.insert(c.id.clone(), n);
        }
        let mut items: Vec<(&Comp, Vec<Hole>)> = self
            .circ
            .componentes
            .iter()
            .filter(|c| visible.as_ref().is_none_or(|v| v.contains(&c.id)))
            .filter(|c| !self.circ.oculta(c))
            .filter_map(|c| c.holes().ok().map(|h| (c, h)))
            .collect();
        let focus = if assembly { None } else { self.grupo.clone() };
        let en_foco = |c: &Comp| focus.as_deref().is_some_and(|f| Self::en_grupo(c, f));
        if self.solo_grupo && focus.is_some() {
            items.retain(|(c, _)| en_foco(c));
        }
        items.sort_by_key(|(c, _)| layer(c.tipo));
        let draw_item = |c: &Comp, hs: &[Hole], focused: bool| {
            let look = draw::Look {
                selected: !assembly
                    && (self.selected.as_deref() == Some(c.id.as_str())
                        || self.sel.contains(&c.id)),
                current: current.contains(&c.id),
                focus: focused,
                t,
            };
            draw::comp(
                &painter,
                &view,
                c,
                hs,
                simref,
                look,
                fidx.get(&c.id).copied().unwrap_or(0),
            );
        };
        for (c, hs) in &items {
            draw_item(c, hs, false);
        }
        if focus.is_some() && !self.solo_grupo {
            painter.rect_filled(rect, 0.0, Color32::from_black_alpha(135));
            for (c, hs) in items.iter().filter(|(c, _)| en_foco(c)) {
                draw_item(c, hs, true);
            }
        }
        if let Some(f) = &focus {
            let nombre = if f.is_empty() {
                "Sin grupo".to_string()
            } else {
                f.clone()
            };
            let n = self.ids_de(f).len();
            let txt = format!("Enfocando {nombre} · {n} pieza(s)  ·  clic o Esc para salir");
            let g = painter.layout_no_wrap(txt, egui::FontId::proportional(12.5), Color32::WHITE);
            let r = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(12.0, 38.0),
                g.size() + egui::vec2(18.0, 10.0),
            );
            painter.rect_filled(r, 7.0, Color32::from_rgba_unmultiplied(20, 26, 33, 235));
            painter.rect_stroke(
                r,
                7.0,
                egui::Stroke::new(1.0_f32, ACCENT),
                egui::StrokeKind::Inside,
            );
            painter.galley(r.min + egui::vec2(9.0, 5.0), g, Color32::WHITE);
            if ui
                .interact(r, ui.id().with("quitar_foco"), egui::Sense::click())
                .clicked()
            {
                self.grupo = None;
                self.solo_grupo = false;
                self.status = "Enfoque quitado.".into();
            }
        }
        let under = hover.and_then(|m| {
            items
                .iter()
                .rev()
                .filter_map(|(c, hs)| {
                    draw::hit(&view, c, hs, m, fidx.get(&c.id).copied().unwrap_or(0))
                        .map(|d| (d, c.id.clone()))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|x| x.1)
        });

        if !assembly {
            if let Tool::Place(tp) = self.tool {
                if tp == Tipo::Chip {
                    if let Some(Hole::Main { board, col, .. }) = hh {
                        draw::chip_ghost(&painter, &view, board, col.min(self.circ.columnas - 6));
                    }
                } else if let (Some(a), Some(m)) = (self.pending, hover) {
                    let to = hh.map(|h| view.h(h)).unwrap_or(m);
                    painter.extend(egui::Shape::dashed_line(
                        &[view.h(a), to],
                        egui::Stroke::new(0.15 * s, Color32::YELLOW),
                        0.5 * s,
                        0.3 * s,
                    ));
                    painter.circle_stroke(
                        view.h(a),
                        0.45 * s,
                        egui::Stroke::new(0.12 * s, Color32::YELLOW),
                    );
                }
            }
        }

        if resp.hovered() && !assembly {
            let icon = if self.drag.is_some() {
                egui::CursorIcon::Grabbing
            } else {
                match self.tool {
                    Tool::Place(_) => egui::CursorIcon::Crosshair,
                    Tool::Delete => egui::CursorIcon::NotAllowed,
                    Tool::Select => egui::CursorIcon::Default,
                }
            };
            ui.ctx().set_cursor_icon(icon);
        }

        // Sonda: qué hay en el orificio bajo el ratón
        let mut probe = String::new();
        if let Some(h) = hh {
            probe = format!("{}  ·  {}", h.name(), h.describe());
            if live {
                match self.sim.volt_at(h) {
                    Some((_, true)) => probe += "  ·  sin conexión con la fuente",
                    Some((v, false)) => {
                        probe += &format!(
                            "  ·  {:.2} V ({})",
                            v,
                            if v >= 2.5 { "ALTO = 1" } else { "BAJO = 0" }
                        )
                    }
                    None => probe += "  ·  vacío",
                }
            }
            if let Some((c, i)) = items
                .iter()
                .find_map(|(c, hs)| hs.iter().position(|x| *x == h).map(|i| (c, i)))
            {
                probe += &format!("  ·  {}: {}", c.id, c.pin_label(i));
            }
        } else if let Some(id) = &under {
            if let Some(c) = self.circ.find(id) {
                probe = format!("{} · {}", c.id, c.tipo.nombre());
            }
        }
        let tag = |at: egui::Pos2, txt: &str, bottom: bool| {
            let g = painter.layout(
                txt.to_string(),
                egui::FontId::proportional(14.0),
                Color32::WHITE,
                rect.width() - 40.0,
            );
            let pos = if bottom {
                at - egui::vec2(0.0, g.size().y)
            } else {
                at
            };
            painter.rect_filled(
                egui::Rect::from_min_size(
                    pos - egui::vec2(8.0, 5.0),
                    g.size() + egui::vec2(16.0, 10.0),
                ),
                6.0,
                Color32::from_rgba_unmultiplied(10, 20, 15, 215),
            );
            painter.galley(pos, g, Color32::WHITE);
        };
        if !probe.is_empty() {
            tag(rect.left_top() + egui::vec2(14.0, 10.0), &probe, false);
        }
        if !assembly {
            tag(
                rect.left_bottom() + egui::vec2(14.0, -10.0),
                &self.hint(),
                true,
            );
        }

        if assembly {
            return;
        }
        let mut act = None;
        if resp.clicked() {
            act = match self.tool {
                Tool::Select if ui.input(|i| i.modifiers.shift || i.modifiers.command) => {
                    under.clone().map(Act::Toggle)
                }
                Tool::Select => Some(under.clone().map(Act::Select).unwrap_or(Act::Deselect)),
                Tool::Delete => under.clone().map(Act::Delete),
                Tool::Place(tp) => hh.map(|h| Act::Place(tp, h)),
            };
        }
        if resp.secondary_clicked() {
            act = Some(if self.pending.is_some() {
                Act::Cancel
            } else {
                under.clone().map(Act::Delete).unwrap_or(Act::Cancel)
            });
        }
        drop(items);
        if let Some(a) = act {
            self.apply(a);
        }
    }

    fn tool_btn(&mut self, ui: &mut egui::Ui, t: Tool, label: &str, tip: &str) {
        let r = ui.add_sized(
            [ui.available_width(), 28.0],
            egui::SelectableLabel::new(self.tool == t, RichText::new(label).size(15.0)),
        );
        if r.on_hover_text(tip).clicked() {
            self.tool = t;
            self.pending = None;
        }
    }

    fn tools_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.lado == LadoPanel::Piezas,
                    RichText::new("Piezas").size(14.0),
                )
                .clicked()
            {
                self.lado = LadoPanel::Piezas;
            }
            if ui
                .selectable_label(
                    self.lado == LadoPanel::Grupos,
                    RichText::new("Grupos").size(14.0),
                )
                .clicked()
            {
                self.lado = LadoPanel::Grupos;
            }
        });
        ui.add_space(4.0);
        match self.lado {
            LadoPanel::Piezas => self.piezas_panel(ui),
            LadoPanel::Grupos => self.grupos_panel(ui),
        }
    }

    fn piezas_panel(&mut self, ui: &mut egui::Ui) {
        section(ui, "Placa");
        ui.horizontal(|ui| {
            ui.label("Placas:");
            let mut t = self.circ.tableros;
            egui::ComboBox::from_id_salt("tableros")
                .width(60.0)
                .selected_text(t.to_string())
                .show_ui(ui, |ui| {
                    for n in 1..=MAX_TABLEROS {
                        ui.selectable_value(&mut t, n, n.to_string());
                    }
                });
            if t != self.circ.tableros {
                self.modify(|c| c.tableros = t);
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Columnas:");
            let mut n = self.circ.columnas;
            egui::ComboBox::from_id_salt("columnas")
                .width(60.0)
                .selected_text(n.to_string())
                .show_ui(ui, |ui| {
                    for x in [30, MAX_COLS] {
                        ui.selectable_value(&mut n, x, x.to_string());
                    }
                });
            if n != self.circ.columnas {
                self.modify(|c| c.columnas = n);
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
            }
        });
        self.tool_btn(
            ui,
            Tool::Select,
            "Seleccionar / accionar",
            "Ver datos de una pieza o cambiar un interruptor",
        );
        self.tool_btn(
            ui,
            Tool::Delete,
            "Quitar pieza",
            "Haz clic en una pieza para quitarla",
        );
        section(ui, "Piezas");
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Cable),
            "Cable puente",
            "Une dos orificios",
        );
        egui::ComboBox::from_id_salt("wire")
            .selected_text(format!("Color: {}", self.wire_color))
            .show_ui(ui, |ui| {
                for c in WIRE_COLORS {
                    ui.selectable_value(&mut self.wire_color, c.to_string(), *c);
                }
            });
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Resistencia),
            "Resistencia",
            "Limita la corriente",
        );
        egui::ComboBox::from_id_salt("res")
            .selected_text(format!("Valor: {}", fmt_ohm(self.res_val)))
            .show_ui(ui, |ui| {
                for v in RES_VALS {
                    ui.selectable_value(&mut self.res_val, *v, fmt_ohm(*v));
                }
            });
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Led),
            "LED",
            "Diodo que se enciende con corriente",
        );
        egui::ComboBox::from_id_salt("led")
            .selected_text(format!("Color: {}", self.led_color))
            .show_ui(ui, |ui| {
                for c in LED_COLORS {
                    ui.selectable_value(&mut self.led_color, c.to_string(), *c);
                }
            });
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Interruptor),
            "Interruptor",
            "Abre o cierra el paso de corriente",
        );
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Display),
            "Display 7 segmentos",
            "Módulo de cátodo común: 8 terminales (a-g y común)",
        );
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Etiqueta),
            "Etiqueta de red",
            "Marca un punto con un nombre; las etiquetas con el mismo nombre se conectan",
        );
        ui.horizontal(|ui| {
            ui.label("Red:");
            ui.add(
                egui::TextEdit::singleline(&mut self.label_name)
                    .desired_width(80.0)
                    .hint_text("p1"),
            );
        });
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Chip),
            "Chip de compuertas",
            "Circuito integrado 74HC de 14 patas",
        );
        egui::ComboBox::from_id_salt("chip")
            .selected_text(&self.chip_model)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for c in CHIPS {
                    ui.selectable_value(
                        &mut self.chip_model,
                        c.modelo.to_string(),
                        format!("{} - {}", c.modelo, c.nombre),
                    );
                }
            });
        self.tool_btn(
            ui,
            Tool::Place(Tipo::Fuente),
            "Fuente 5 V",
            "Alimentación: + y -",
        );
        section(ui, "Etiquetas y cables");
        let todos = |c: &Circuito, t: Tipo| {
            c.componentes
                .iter()
                .filter(|x| x.tipo == t)
                .map(|x| x.id.clone())
                .collect::<Vec<_>>()
        };
        let (et, ca) = (
            todos(&self.circ, Tipo::Etiqueta),
            todos(&self.circ, Tipo::Cable),
        );
        if ui
            .add_enabled(
                !et.is_empty(),
                egui::Button::new("Todas las etiquetas a cables"),
            )
            .clicked()
        {
            self.convertir_varios(&et);
        }
        if ui
            .add_enabled(
                !ca.is_empty(),
                egui::Button::new("Todos los cables a etiquetas"),
            )
            .clicked()
        {
            self.convertir_varios(&ca);
        }
        if ui
            .add_enabled(
                !ca.is_empty(),
                egui::Button::new("Ordenar todos los cables"),
            )
            .on_hover_text("Tramos rectos que esquivan las piezas")
            .clicked()
        {
            self.ordenar(ca);
        }
        ui.label(RichText::new("Ctrl o Shift + clic: seleccionar varias. Shift + arrastrar: seleccionar con un recuadro.").color(MUTED));
        section(ui, "Consejos");
        ui.label("• Pasa el ratón por un orificio: verás su voltaje y, en verde, todo lo que está conectado con él.");
        ui.label("• Rueda del ratón: acercar; arrastra: mover; doble clic: ver todo.");
        ui.label("• Clic derecho sobre una pieza para quitarla. Esc cancela. Ctrl+Z deshace.");
        ui.label("• Ctrl+Shift+Z o Ctrl+Y rehace; F centra la selección o el grupo.");
        ui.label("• Pestaña Grupos: crea carpetas, enfoca, oculta y reordena cables por grupo.");
        ui.label("• El circuito se guarda solo; un agente de IA puede editarlo (pestaña «Agentes de IA»).");
    }

    fn grupos_panel(&mut self, ui: &mut egui::Ui) {
        let carpetas = self.circ.todas_carpetas();
        let counts: HashMap<String, usize> = carpetas
            .iter()
            .map(|f| (f.clone(), self.circ.contenido(f).len()))
            .collect();
        let sin_grupo = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.carpeta.is_none())
            .count();
        let activo = self.grupo.clone();
        let ocultas = self.circ.ocultas.clone();

        let mut seleccionar: Option<String> = None;
        let mut enfocar: Option<String> = None;
        let mut centrar: Option<String> = None;
        let mut ocultar: Option<String> = None;
        let mut borrar: Option<String> = None;

        section(ui, "Árbol de grupos");
        card(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.selectable_label(
                    activo.as_deref() == Some(""),
                    RichText::new("Sin grupo").size(13.0),
                );
                if r.double_clicked() {
                    enfocar = Some(String::new());
                    centrar = Some(String::new());
                } else if r.clicked() {
                    seleccionar = Some(String::new());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let focused = activo.as_deref() == Some("");
                    if ui
                        .add(
                            egui::Button::new(RichText::new("foco").small().color(if focused {
                                ACCENT
                            } else {
                                MUTED
                            }))
                            .frame(false),
                        )
                        .on_hover_text("Enfocar las piezas sin grupo")
                        .clicked()
                    {
                        enfocar = Some(String::new());
                    }
                    ui.label(RichText::new(sin_grupo.to_string()).small().color(MUTED));
                });
            });

            for f in &carpetas {
                if padre_colapsado(f, &self.colapsadas) {
                    continue;
                }
                let depth = f.matches('/').count();
                let has_hijos = carpetas.iter().any(|x| x.starts_with(&format!("{f}/")));
                let count = counts.get(f).copied().unwrap_or(0);
                let nombre = f.rsplit('/').next().unwrap_or(f);
                let oculto_padre = ocultas.iter().any(|x| x != f && dentro(f, x));
                let hidden = ocultas.iter().any(|x| x == f);
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 10.0);
                    if has_hijos {
                        let abierta = !self.colapsadas.contains(f);
                        if ui.small_button(if abierta { "v" } else { ">" }).clicked() {
                            if abierta {
                                self.colapsadas.insert(f.clone());
                            } else {
                                self.colapsadas.remove(f);
                            }
                        }
                    } else {
                        ui.add_space(18.0);
                    }
                    if ui
                        .small_button(if hidden { "oculto" } else { "ver" })
                        .on_hover_text(if hidden {
                            "Mostrar esta carpeta"
                        } else {
                            "Ocultar esta carpeta"
                        })
                        .clicked()
                    {
                        ocultar = Some(f.clone());
                    }
                    let color = if oculto_padre {
                        MUTED
                    } else {
                        Color32::from_rgb(226, 232, 240)
                    };
                    let r = ui.selectable_label(
                        activo.as_deref() == Some(f.as_str()),
                        RichText::new(nombre).color(color).size(13.0),
                    );
                    if r.double_clicked() {
                        enfocar = Some(f.clone());
                        centrar = Some(f.clone());
                    } else if r.clicked() {
                        seleccionar = Some(f.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if count == 0
                            && !has_hijos
                            && ui
                                .small_button("x")
                                .on_hover_text("Borrar carpeta vacía")
                                .clicked()
                        {
                            borrar = Some(f.clone());
                        }
                        let focused = activo.as_deref() == Some(f.as_str());
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("foco").small().color(if focused {
                                        ACCENT
                                    } else {
                                        MUTED
                                    }),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Enfocar este grupo")
                            .clicked()
                        {
                            enfocar = Some(f.clone());
                        }
                        ui.label(RichText::new(count.to_string()).small().color(MUTED));
                    });
                });
            }
            if carpetas.is_empty() {
                ui.label(
                    RichText::new(
                        "Aún no hay grupos. Crea uno abajo o agrupa los cables por color.",
                    )
                    .small()
                    .color(MUTED),
                );
            }
        });

        if self.grupo.is_some() {
            ui.add_enabled(
                self.grupo.is_some(),
                egui::Checkbox::new(&mut self.solo_grupo, "Solo el grupo enfocado"),
            )
            .on_hover_text("Oculta todo lo que no sea de este grupo");
        }

        section(ui, "Crear carpeta");
        card(ui, |ui| {
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.nueva_carpeta)
                        .hint_text("Cables/rojo")
                        .desired_width(ui.available_width() - 34.0),
                );
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("+").clicked() || enter {
                    let n = self.nueva_carpeta.clone();
                    self.crear_carpeta(&n);
                }
            });
        });

        let ids = self.seleccionados();
        section(ui, "Mover selección");
        card(ui, |ui| {
            ui.label(
                RichText::new(format!("{} pieza(s) seleccionada(s)", ids.len()))
                    .small()
                    .color(MUTED),
            );
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("mover_a")
                    .selected_text(if self.mover_a.is_empty() {
                        "Sin grupo".to_string()
                    } else {
                        self.mover_a.clone()
                    })
                    .width(ui.available_width() - 64.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.mover_a, String::new(), "Sin grupo");
                        for f in &carpetas {
                            ui.selectable_value(&mut self.mover_a, f.clone(), f);
                        }
                    });
                if ui
                    .add_enabled(!ids.is_empty(), egui::Button::new("Mover"))
                    .clicked()
                {
                    let f = if self.mover_a.is_empty() {
                        None
                    } else {
                        Some(self.mover_a.clone())
                    };
                    self.mover_a_carpeta(&ids, f);
                }
            });
        });

        let hay_sueltos = self
            .circ
            .componentes
            .iter()
            .any(|c| c.tipo == Tipo::Cable && c.carpeta.is_none());
        card(ui, |ui| {
            if ui
                .add_enabled(hay_sueltos, egui::Button::new("Agrupar cables por color"))
                .clicked()
            {
                self.agrupar_cables_por_color();
            }
            if ui.button("Mostrar todo").clicked() {
                self.mostrar_todo();
            }
        });

        section(ui, "Separación");
        card(ui, |ui| {
            ui.horizontal(|ui| {
                for (name, v) in [("Compacta", 2.0), ("Normal", 4.0), ("Amplia", 8.0)] {
                    if ui
                        .selectable_label((self.sep_cables - v).abs() < 0.1, name)
                        .clicked()
                    {
                        self.sep_cables = v;
                    }
                }
            });
            if ui.button("Reordenar por grupos").clicked() {
                self.ordenar_grupos(None);
            }
            if let Some(g) = self.grupo.clone() {
                let n = self.ids_de(&g).len();
                if ui
                    .button(format!("Reordenar «{}» ({n})", self.nombre_grupo()))
                    .clicked()
                {
                    self.ordenar_grupos(Some(&g));
                }
            }
        });

        section(ui, "Buscar cable");
        ui.add(
            egui::TextEdit::singleline(&mut self.buscar_cable)
                .hint_text("id, color o carpeta")
                .desired_width(f32::INFINITY),
        );
        let q = self.buscar_cable.trim().to_lowercase();
        let found: Vec<(String, String)> = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .filter(|c| {
                q.is_empty()
                    || format!(
                        "{} {} {}",
                        c.id,
                        c.color_or(""),
                        c.carpeta.clone().unwrap_or_default()
                    )
                    .to_lowercase()
                    .contains(&q)
            })
            .map(|c| (c.id.clone(), format!("{} · {}", c.id, c.color_or("rojo"))))
            .take(50)
            .collect();
        if found.is_empty() {
            ui.label(
                RichText::new(if q.is_empty() {
                    "No hay cables."
                } else {
                    "Sin coincidencias."
                })
                .small()
                .color(MUTED),
            );
        } else {
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for (id, label) in &found {
                        if ui
                            .selectable_label(
                                self.selected.as_deref() == Some(id),
                                RichText::new(label).small(),
                            )
                            .clicked()
                        {
                            self.selected = Some(id.clone());
                            self.sel.clear();
                            self.centrar_ids(std::slice::from_ref(id));
                        }
                    }
                });
        }

        if let Some(f) = seleccionar {
            let nombre = if f.is_empty() {
                "Sin grupo".to_string()
            } else {
                f.clone()
            };
            let ids = self.ids_de(&f);
            self.sel = ids.iter().cloned().collect();
            self.selected = if ids.len() == 1 {
                ids.first().cloned()
            } else {
                None
            };
            self.status = format!("{nombre}: {} pieza(s).", ids.len());
        }
        if let Some(f) = enfocar {
            self.enfocar(&f);
        }
        if let Some(f) = centrar {
            self.centrar_grupo(&f);
        }
        if let Some(f) = ocultar {
            self.toggle_oculta(&f);
        }
        if let Some(f) = borrar {
            self.borrar_carpeta_vacia(&f);
        }
    }

    fn state_panel(&mut self, ui: &mut egui::Ui) {
        if !self.sel.is_empty() {
            let ids: Vec<String> = self
                .circ
                .componentes
                .iter()
                .filter(|c| self.sel.contains(&c.id))
                .map(|c| c.id.clone())
                .collect();
            let n = |t: Tipo| {
                self.circ
                    .componentes
                    .iter()
                    .filter(|c| c.tipo == t && self.sel.contains(&c.id))
                    .count()
            };
            let (ca, et) = (n(Tipo::Cable), n(Tipo::Etiqueta));
            ui.label(
                RichText::new(format!("{} piezas seleccionadas", ids.len()))
                    .strong()
                    .size(15.0),
            );
            ui.label(
                RichText::new(format!(
                    "{ca} cable(s), {et} etiqueta(s), {} otra(s): {}",
                    ids.len() - ca - et,
                    ids.join(", ")
                ))
                .small()
                .color(MUTED),
            );
            let mut quitar = false;
            ui.horizontal_wrapped(|ui| {
                if ca > 0
                    && ui
                        .button(format!("Convertir {ca} cable(s) en etiquetas"))
                        .clicked()
                {
                    self.convertir_varios(&ids);
                }
                if et > 0
                    && ui
                        .button(format!("Convertir {et} etiqueta(s) en cables"))
                        .clicked()
                {
                    let ets: Vec<String> = ids
                        .iter()
                        .filter(|id| self.circ.find(id).is_some_and(|c| c.tipo == Tipo::Etiqueta))
                        .cloned()
                        .collect();
                    self.convertir_varios(&ets);
                }
                if ca > 0 && ui.button("Ordenar estos cables (rectos)").clicked() {
                    self.ordenar(ids.clone());
                }
                if ui.button("Quitar las seleccionadas").clicked() {
                    quitar = true;
                }
                if ui.button("Quitar selección").clicked() {
                    self.sel.clear();
                    self.selected = None;
                }
            });
            if quitar {
                let set = std::mem::take(&mut self.sel);
                self.modify(|c| c.componentes.retain(|x| !set.contains(&x.id)));
                self.status = format!("Quitadas {} piezas.", set.len());
            }
            ui.separator();
        }
        if let Some(c) = self
            .selected
            .as_ref()
            .and_then(|id| self.circ.find(id))
            .cloned()
        {
            ui.heading(format!("{} · {}", c.id, c.tipo.nombre()));
            if let Ok(hs) = c.holes() {
                if c.tipo == Tipo::Chip {
                    ui.label(format!(
                        "Pin 1 en {}, pin 14 en {}",
                        hs[0].name(),
                        hs[13].name()
                    ));
                } else {
                    for (i, h) in hs.iter().enumerate() {
                        ui.label(format!("{}: {}", c.pin_label(i), h.name()));
                    }
                }
            }
            let mut n = c.clone();
            ui.horizontal(|ui| {
                ui.label("Carpeta:");
                let actual = n.carpeta.clone().unwrap_or_else(|| "Sin grupo".into());
                egui::ComboBox::from_id_salt("selcarpeta")
                    .selected_text(actual)
                    .width(ui.available_width() - 12.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(n.carpeta.is_none(), "Sin grupo")
                            .clicked()
                        {
                            n.carpeta = None;
                        }
                        for f in self.circ.todas_carpetas() {
                            if ui
                                .selectable_label(n.carpeta.as_deref() == Some(f.as_str()), &f)
                                .clicked()
                            {
                                n.carpeta = Some(f.clone());
                            }
                        }
                    });
            });
            match c.tipo {
                Tipo::Resistencia => {
                    let mut v = c.ohmios();
                    egui::ComboBox::from_id_salt("selres")
                        .selected_text(fmt_ohm(v))
                        .show_ui(ui, |ui| {
                            for x in RES_VALS {
                                ui.selectable_value(&mut v, *x, fmt_ohm(*x));
                            }
                        });
                    ui.label(format!("Bandas: {}", instr::bandas(v).join(", ")));
                    n.valor = Some(v);
                }
                Tipo::Led | Tipo::Cable => {
                    let mut col = c.color_or("rojo");
                    let list = if c.tipo == Tipo::Led {
                        LED_COLORS
                    } else {
                        WIRE_COLORS
                    };
                    egui::ComboBox::from_id_salt("selcol")
                        .selected_text(&col)
                        .show_ui(ui, |ui| {
                            for x in list {
                                ui.selectable_value(&mut col, x.to_string(), *x);
                            }
                        });
                    n.color = Some(col);
                }
                Tipo::Interruptor => {
                    ui.checkbox(&mut n.cerrado, "Cerrado (deja pasar la corriente)");
                }
                Tipo::Chip => {
                    let mut m = c.modelo.clone().unwrap_or_default();
                    egui::ComboBox::from_id_salt("selchip")
                        .selected_text(&m)
                        .show_ui(ui, |ui| {
                            for x in CHIPS {
                                ui.selectable_value(
                                    &mut m,
                                    x.modelo.to_string(),
                                    format!("{} - {}", x.modelo, x.nombre),
                                );
                            }
                        });
                    if let Some(info) = chip_info(&m) {
                        ui.label(format!(
                            "Pin 14 = VCC (+), pin 7 = GND (-); {}.",
                            info.pinout()
                        ));
                    }
                    n.modelo = Some(m);
                }
                Tipo::Fuente => {
                    let mut v = c.voltios();
                    egui::ComboBox::from_id_salt("selv")
                        .selected_text(format!("{v:.1} V"))
                        .show_ui(ui, |ui| {
                            for x in VOLTS {
                                ui.selectable_value(&mut v, *x, format!("{x:.1} V"));
                            }
                        });
                    ui.add(
                        egui::DragValue::new(&mut v)
                            .range(0.5..=24.0)
                            .speed(0.05)
                            .max_decimals(2)
                            .suffix(" V"),
                    )
                    .on_hover_text("Arrastra o escribe el voltaje (0,5 a 24 V)");
                    n.valor = Some((v * 100.0).round() / 100.0);
                }
                Tipo::Display => {
                    ui.label("Display de 7 segmentos de cátodo común.");
                    ui.label("Terminales: a, b, c, d, e, f, g y común. El común va a GND.");
                }
                Tipo::Etiqueta => {
                    ui.label("Nombre de la red (las etiquetas con el mismo nombre se conectan):");
                    let mut name = c.red();
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(160.0));
                    n.nombre = Some(name);
                }
            }
            let mut convertir = false;
            if c.tipo == Tipo::Cable {
                convertir = ui
                    .button("Convertir en etiquetas")
                    .on_hover_text("Cambia el cable por dos etiquetas con el mismo nombre")
                    .clicked();
                if !c.puntos.is_empty() && ui.button("Enderezar cable").clicked() {
                    n.puntos.clear();
                }
                ui.label(
                    RichText::new(
                        "Arrastra el cable para doblarlo; doble clic en un punto blanco lo quita.",
                    )
                    .color(Color32::GRAY),
                );
            }
            if c.tipo == Tipo::Etiqueta {
                convertir = ui
                    .button("Convertir en cable(s)")
                    .on_hover_text("Une con cables todas las etiquetas de esta red")
                    .clicked();
            }
            let quitar = ui.button("Quitar esta pieza").clicked();
            if n != c {
                self.modify(|k| {
                    if let Some(x) = k.componentes.iter_mut().find(|x| x.id == n.id) {
                        *x = n;
                    }
                });
            }
            if quitar {
                self.delete(&c.id);
            }
            if convertir {
                self.convertir(&c.id);
            }
            ui.separator();
        }

        section(ui, "Estado");
        let (estado_color, resumen) = if !self.sim.errors.is_empty() {
            (DANGER, format!("{} error(es)", self.sim.errors.len()))
        } else if !self.sim.warnings.is_empty() {
            (WARN, format!("{} aviso(s)", self.sim.warnings.len()))
        } else {
            (SUCCESS, "Sin problemas detectados".to_string())
        };
        ui.horizontal(|ui| {
            dot(ui, estado_color);
            ui.label(RichText::new(resumen).color(estado_color).strong());
        });
        for e in &self.sim.errors {
            ui.label(RichText::new(format!("ERROR: {e}")).small().color(DANGER));
        }
        for w in &self.sim.warnings {
            ui.label(RichText::new(format!("Aviso: {w}")).small().color(WARN));
        }
        if !self.sim.leds.is_empty() {
            section(ui, "LEDs");
            for l in &self.sim.leds {
                let col = self
                    .circ
                    .find(&l.id)
                    .map(|c| c.color_or("rojo"))
                    .unwrap_or_default();
                let color = draw::color(&col);
                let (txt, tc) = if l.on {
                    (format!("{} · {:.1} mA", l.id, l.ma), color)
                } else {
                    (format!("{} · apagado", l.id), MUTED)
                };
                ui.horizontal(|ui| {
                    dot(ui, if l.on { color } else { MUTED });
                    ui.label(RichText::new(txt).small().color(tc));
                });
            }
        }
        let sws: Vec<(String, bool)> = self
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Interruptor)
            .map(|c| (c.id.clone(), c.cerrado))
            .collect();
        if !sws.is_empty() {
            section(ui, "Interruptores");
            for (id, mut on) in sws {
                let txt = format!("{id} · {}", if on { "ON" } else { "OFF" });
                if ui.checkbox(&mut on, RichText::new(txt).small()).changed() {
                    self.toggle(&id);
                }
            }
        }
        if !self.sim.chips.is_empty() {
            section(ui, "Chips");
            let powered = self.sim.chips.iter().filter(|c| c.powered).count();
            ui.label(
                RichText::new(format!(
                    "{} chips · {} con energía",
                    self.sim.chips.len(),
                    powered
                ))
                .small()
                .color(MUTED),
            );

            egui::CollapsingHeader::new("Detalle")
                .default_open(false)
                .show(ui, |ui| {
                    for ch in &self.sim.chips {
                        if ch.powered {
                            let o: Vec<String> = ch
                                .outputs
                                .iter()
                                .map(|(p, v)| format!("{p}={}", *v as u8))
                                .collect();
                            ui.label(
                                RichText::new(format!(
                                    "{} {} · {}",
                                    ch.id,
                                    ch.modelo,
                                    o.join("  ")
                                ))
                                .small(),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("{} {} · sin energía", ch.id, ch.modelo))
                                    .small()
                                    .color(WARN),
                            );
                        }
                    }
                });
        }
        let tiene_switches = self
            .circ
            .componentes
            .iter()
            .any(|c| c.tipo == Tipo::Interruptor);
        if self.tabla.is_none() && tiene_switches {
            section(ui, "Tabla de verdad");
            ui.label(
                RichText::new(
                    "En circuitos grandes se calcula a petición para no bloquear la ventana.",
                )
                .small()
                .color(MUTED),
            );
            if ui.button("Calcular tabla de verdad").clicked() {
                let t = sim::tabla(&self.circ);
                self.tabla = t;
                self.pasos = instr::instrucciones(&self.circ, &self.sim, self.tabla.as_ref());
            }
        }
        if let Some(t) = &self.tabla {
            section(ui, "Tabla de verdad");
            egui::CollapsingHeader::new(format!("{} combinaciones", t.filas.len()))
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            egui::Grid::new("tabla").striped(true).show(ui, |ui| {
                                for s in t.sw.iter().chain(&t.leds).chain(&t.displays) {
                                    ui.label(RichText::new(s).strong().small());
                                }
                                ui.end_row();
                                for (ri, (a, b)) in t.filas.iter().enumerate() {
                                    for x in a {
                                        ui.label(if *x { "ON" } else { "OFF" });
                                    }
                                    for x in b {
                                        ui.label(if *x {
                                            RichText::new("ON").color(SUCCESS)
                                        } else {
                                            RichText::new("OFF").color(MUTED)
                                        });
                                    }
                                    for ch in &t.disp_filas[ri] {
                                        ui.label(
                                            RichText::new(ch.to_string())
                                                .strong()
                                                .color(Color32::from_rgb(255, 140, 90)),
                                        );
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                });
        }
    }

    fn assembly_panels(&mut self, ctx: &egui::Context) {
        let n = self.pasos.len();
        egui::SidePanel::left("pasos")
            .resizable(false)
            .exact_width(270.0)
            .show(ctx, |ui| {
                ui.heading("Pasos de armado");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut go = None;
                    for (i, p) in self.pasos.iter().enumerate() {
                        let txt = RichText::new(format!("{}. {}", i + 1, p.titulo));
                        let txt = if i < self.paso {
                            txt.color(Color32::from_rgb(120, 190, 130))
                        } else {
                            txt
                        };
                        if ui.selectable_label(self.paso == i, txt).clicked() {
                            go = Some(i);
                        }
                    }
                    if let Some(i) = go {
                        self.paso = i;
                    }
                });
            });
        egui::SidePanel::right("detalle")
            .resizable(false)
            .exact_width(390.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Paso {} de {}", self.paso + 1, n))
                        .color(Color32::from_rgb(140, 200, 255)),
                );
                ui.add(egui::ProgressBar::new(
                    (self.paso + 1) as f32 / n.max(1) as f32,
                ));
                if let Some(p) = self.pasos.get(self.paso) {
                    ui.heading(RichText::new(&p.titulo).size(20.0));
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height() - 110.0)
                        .show(ui, |ui| {
                            for t in &p.texto {
                                ui.label(RichText::new(format!("• {t}")).size(15.0));
                                ui.add_space(2.0);
                            }
                        });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.paso > 0,
                            egui::Button::new(RichText::new("< Anterior").size(16.0)),
                        )
                        .clicked()
                    {
                        self.paso -= 1;
                    }
                    if ui
                        .add_enabled(
                            self.paso + 1 < n,
                            egui::Button::new(RichText::new("Siguiente >").size(16.0)),
                        )
                        .clicked()
                    {
                        self.paso += 1;
                    }
                });
                ui.label("También puedes usar las flechas del teclado.");
                if ui.button("Guardar instrucciones en un archivo").clicked() {
                    let path = self.file.with_file_name("instrucciones_armado.md");
                    self.status = match std::fs::write(
                        &path,
                        instr::markdown(&self.circ.nombre, &self.pasos),
                    ) {
                        Ok(()) => format!("Instrucciones guardadas en {}", path.display()),
                        Err(e) => format!("No pude guardar: {e}"),
                    };
                }
            });
    }

    fn ia_panel(&mut self, ui: &mut egui::Ui) {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "opencircuit".into());
        let file = self.file.display().to_string();
        let code = |ui: &mut egui::Ui, txt: &str| {
            ui.horizontal(|ui| {
                let mut s = txt;
                ui.add(
                    egui::TextEdit::multiline(&mut s)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(ui.available_width() - 90.0),
                );
                if ui.button("Copiar").clicked() {
                    ui.ctx().copy_text(txt.to_string());
                }
            });
        };
        ui.heading("Arma circuitos con ayuda de una IA");
        ui.label("Describe lo que quieres (por ejemplo: «un LED que se encienda solo cuando dos botones estén pulsados») y deja que la IA lo arme. El circuito aparece aquí en vivo, con su simulación y sus instrucciones de armado.");
        ui.add_space(8.0);

        ui.label(
            RichText::new(
                "Opción 1 · Agentes con MCP (Claude Desktop, Claude Code, Cursor y otros)",
            )
            .strong()
            .size(16.0),
        );
        ui.label("El agente obtiene herramientas para ver la guía, añadir y quitar piezas, cambiar interruptores, simular y pedir las instrucciones. Cada cambio se guarda en el archivo compartido y esta ventana se actualiza sola.");
        ui.label("Claude Code: ejecuta esta línea en una terminal:");
        code(
            ui,
            &format!("claude mcp add protoboard -- \"{exe}\" --mcp --archivo \"{file}\""),
        );
        ui.label(
            "Claude Desktop, Cursor u otros: añade esto a su configuración de servidores MCP:",
        );
        let cfg = serde_json::json!({"mcpServers": {"protoboard": {"command": exe, "args": ["--mcp", "--archivo", file]}}});
        code(ui, &serde_json::to_string_pretty(&cfg).unwrap());
        ui.add_space(10.0);

        ui.label(
            RichText::new("Opción 2 · Cualquier chat de IA (ChatGPT, Gemini, Claude en la web...)")
                .strong()
                .size(16.0),
        );
        ui.horizontal(|ui| {
            ui.label("1. Copia la guía del formato y pégala en el chat junto con tu pedido:");
            if ui.button("Copiar guía para la IA").clicked() {
                ui.ctx().copy_text(format!(
                    "{}\n\nResponde solo con el JSON del circuito.\n\nMi pedido: ",
                    guia()
                ));
                self.pegar_msg =
                    "Guía copiada. Pégala en el chat y escribe tu pedido al final.".into();
            }
        });
        ui.label("2. Pega aquí la respuesta de la IA (puede incluir texto: se busca el JSON) y pulsa «Cargar circuito»:");
        ui.add(
            egui::TextEdit::multiline(&mut self.pegar)
                .desired_rows(7)
                .desired_width(f32::INFINITY)
                .code_editor()
                .hint_text("{ \"nombre\": \"...\", \"componentes\": [ ... ] }"),
        );
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Cargar circuito").size(15.0)).clicked() {
                match parse_text(&self.pegar) {
                    Ok(c) => {
                        let errs = c.validate();
                        let n = c.componentes.len();
                        self.set_circuit(c, format!("Circuito de la IA cargado ({n} piezas)."));
                        self.pegar_msg = if errs.is_empty() {
                            format!("Cargado: {n} piezas, sin errores. Míralo en la pestaña de protoboard.")
                        } else {
                            format!("Cargado con problemas (pídele a la IA que los corrija):\n- {}", errs.join("\n- "))
                        };
                    }
                    Err(e) => self.pegar_msg = e,
                }
            }
            if ui.button("Copiar circuito actual (para pedir cambios)").clicked() {
                ui.ctx().copy_text(serde_json::to_string_pretty(&self.circ).unwrap());
                self.pegar_msg = "Circuito copiado.".into();
            }
        });
        if !self.pegar_msg.is_empty() {
            ui.label(RichText::new(&self.pegar_msg).color(Color32::from_rgb(140, 200, 255)));
        }
        ui.add_space(10.0);
        ui.label(RichText::new("Archivo compartido").strong().size(16.0));
        ui.label(format!("{file}\nCualquier programa o agente que escriba este archivo (formato descrito en la guía) cambia el circuito de esta ventana."));
        ui.label("Desde la terminal también puedes usar: --simular <archivo>, --instrucciones <archivo> y --guia.");
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_file(ctx);
        if !ctx.wants_keyboard_input() {
            let (esc, undo, redo, del, right, left, centrar) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Escape),
                    i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                    i.modifiers.command
                        && (i.key_pressed(egui::Key::Y)
                            || (i.modifiers.shift && i.key_pressed(egui::Key::Z))),
                    i.key_pressed(egui::Key::Delete),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::F),
                )
            });
            if esc {
                if self.pending.is_some() {
                    self.pending = None;
                } else if self.tool == Tool::Select && self.grupo.is_some() {
                    self.grupo = None;
                    self.solo_grupo = false;
                    self.status = "Enfoque quitado.".into();
                } else {
                    self.tool = Tool::Select;
                }
            }
            if undo {
                self.undo_last();
            }
            if redo {
                self.redo_last();
            }
            if centrar {
                if let Some(g) = self.grupo.clone() {
                    self.centrar_grupo(&g);
                } else if !self.sel.is_empty() {
                    let ids: Vec<String> = self.sel.iter().cloned().collect();
                    self.centrar_ids(&ids);
                    self.status = format!("Centradas {} piezas.", ids.len());
                } else if let Some(id) = self.selected.clone() {
                    self.centrar_ids(std::slice::from_ref(&id));
                }
            }
            if del && self.tab == Tab::Protoboard {
                if let Some(id) = self.selected.clone() {
                    self.delete(&id);
                }
            }
            if self.tab == Tab::Armado {
                if right && self.paso + 1 < self.pasos.len() {
                    self.paso += 1;
                }
                if left && self.paso > 0 {
                    self.paso -= 1;
                }
            }
        }

        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("OpenCircuit").strong().size(19.0));
                    ui.add_space(4.0);
                    for (t, name) in [
                        (Tab::Protoboard, "Protoboard y simulación"),
                        (Tab::Armado, "Instrucciones de armado"),
                        (Tab::Ia, "Agentes de IA"),
                    ] {
                        if ui
                            .selectable_label(self.tab == t, RichText::new(name).size(15.0))
                            .clicked()
                        {
                            self.tab = t;
                        }
                    }
                    ui.separator();
                    ui.menu_button(RichText::new("Ejemplos").size(14.0), |ui| {
                        for e in ejemplos() {
                            if ui.button(&e.nombre).clicked() {
                                let n = e.nombre.clone();
                                self.set_circuit(e, format!("Ejemplo «{n}» cargado."));
                                ui.close_menu();
                            }
                        }
                    });
                    if ui
                        .button(RichText::new("Nuevo").size(14.0))
                        .on_hover_text("Protoboard vacía")
                        .clicked()
                    {
                        self.set_circuit(
                            Circuito {
                                nombre: "Mi circuito".into(),
                                ..Default::default()
                            },
                            "Protoboard vacía. Empieza por la fuente.".into(),
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.undo.is_empty(),
                            egui::Button::new(RichText::new("Deshacer").size(14.0)),
                        )
                        .on_hover_text("Ctrl+Z")
                        .clicked()
                    {
                        self.undo_last();
                    }
                    if ui
                        .add_enabled(
                            !self.redo.is_empty(),
                            egui::Button::new(RichText::new("Rehacer").size(14.0)),
                        )
                        .on_hover_text("Ctrl+Shift+Z o Ctrl+Y")
                        .clicked()
                    {
                        self.redo_last();
                    }
                    ui.separator();
                    ui.label(RichText::new("Nombre:").small().color(MUTED));
                    let mut nombre = self.circ.nombre.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut nombre).desired_width(180.0))
                        .lost_focus()
                        && nombre != self.circ.nombre
                    {
                        self.modify(|c| c.nombre = nombre);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.shot.is_some() {
                            ui.label(
                                RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                    .small()
                                    .color(MUTED),
                            );
                        } else {
                            ui.label(RichText::new("Guardado").small().color(SUCCESS))
                                .on_hover_text(self.file.display().to_string());
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).small().color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut info = format!(
                            "{} piezas · zoom {:.0}%",
                            self.circ.componentes.len(),
                            self.zoom * 100.0
                        );
                        if !self.sel.is_empty() {
                            info.push_str(&format!(" · {} sel.", self.sel.len()));
                        }
                        if let Some(g) = &self.grupo {
                            let n = if g.is_empty() {
                                "Sin grupo".to_string()
                            } else {
                                g.clone()
                            };
                            info.push_str(&format!(" · foco: {n}"));
                        }
                        ui.label(RichText::new(info).small().color(MUTED));
                    });
                });
            });

        match self.tab {
            Tab::Protoboard => {
                egui::SidePanel::left("tools")
                    .resizable(false)
                    .exact_width(260.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL)
                            .inner_margin(egui::Margin::symmetric(10, 8)),
                    )
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.tools_panel(ui));
                    });
                egui::SidePanel::right("estado")
                    .resizable(false)
                    .exact_width(340.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL)
                            .inner_margin(egui::Margin::symmetric(10, 8)),
                    )
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.state_panel(ui));
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::new())
                    .show(ctx, |ui| self.board(ui, false));
            }
            Tab::Armado => {
                self.assembly_panels(ctx);
                egui::CentralPanel::default()
                    .frame(egui::Frame::new())
                    .show(ctx, |ui| self.board(ui, true));
            }
            Tab::Ia => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.ia_panel(ui));
                });
            }
        }

        if let Some(path) = self.shot.clone() {
            self.frames += 1;
            if self.frames == 120 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let img = ctx.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = img {
                if let Err(e) = save_png(&path, &img) {
                    eprintln!("No pude guardar la captura: {e}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if self.frames > 900 {
                eprintln!("No llegó la captura de pantalla");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        } else if self.tab == Tab::Armado {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(700));
        }
    }
}

impl App {
    fn convertir(&mut self, id: &str) {
        self.convertir_varios(&[id.to_string()]);
    }

    fn ordenar(&mut self, ids: Vec<String>) {
        let n = ids.len();
        self.modify(|c| route::ordenar(c, &ids));
        self.status = format!("{n} cable(s) ordenados con tramos rectos.");
    }

    /// Cables -> dos etiquetas cada uno (con su color); etiquetas -> cables que unen su red
    /// completa, del color de la etiqueta y trazados rectos esquivando piezas. Todo de una vez.
    fn convertir_varios(&mut self, ids: &[String]) {
        let mut k = self.circ.clone();
        let (mut n_et, mut nuevos, mut avisos) = (0, Vec::new(), Vec::new());
        for id in ids {
            let Some(c) = k.find(id).cloned().filter(|c| c.tipo == Tipo::Cable) else {
                continue;
            };
            let Ok(hs) = c.holes() else { continue };
            let usados: HashSet<String> = k
                .componentes
                .iter()
                .filter(|x| x.tipo == Tipo::Etiqueta)
                .map(|x| x.red())
                .collect();
            let red = (0..)
                .map(|i| {
                    if i == 0 {
                        c.id.clone()
                    } else {
                        format!("{}_{i}", c.id)
                    }
                })
                .find(|r| !usados.contains(r))
                .unwrap();
            k.componentes.retain(|x| x.id != c.id);
            for h in &hs {
                let mut e = Comp::nuevo(
                    &k.next_id(Tipo::Etiqueta.prefix()),
                    Tipo::Etiqueta,
                    &[&h.name()],
                );
                e.nombre = Some(red.clone());
                e.color = c.color.clone();
                e.carpeta = c.carpeta.clone();
                k.componentes.push(e);
            }
            n_et += 1;
        }
        let mut redes: Vec<String> = Vec::new();
        for id in ids {
            if let Some(c) = self.circ.find(id).filter(|c| c.tipo == Tipo::Etiqueta) {
                if !redes.contains(&c.red()) {
                    redes.push(c.red());
                }
            }
        }
        for red in &redes {
            match unir_red(&k, red) {
                Ok((nk, ids_red)) => {
                    k = nk;
                    nuevos.extend(ids_red);
                }
                Err(e) => avisos.push(e),
            }
        }
        route::ordenar(&mut k, &nuevos);
        self.status = format!(
            "{n_et} cable(s) pasados a etiquetas; {} cable(s) nuevos desde etiquetas.{}",
            nuevos.len(),
            if avisos.is_empty() {
                String::new()
            } else {
                format!(" Sin cambiar: {}.", avisos.join("; "))
            }
        );
        self.selected = None;
        self.sel.clear();
        if k != self.circ {
            self.modify(|x| *x = k);
        }
    }

    fn undo_last(&mut self) {
        if let Some(c) = self.undo.pop() {
            self.redo.push(self.circ.clone());
            self.circ = c;
            self.recompute();
            self.save();
            self.status = "Deshecho.".into();
        }
    }

    fn redo_last(&mut self) {
        if let Some(c) = self.redo.pop() {
            self.undo.push(self.circ.clone());
            self.circ = c;
            self.recompute();
            self.save();
            self.status = "Rehecho.".into();
        }
    }
}

/// Cambia todas las etiquetas de la red `red` por cables del color de la etiqueta.
/// Une cada etiqueta con la más cercana ya conectada; cada cable sale de un orificio libre
/// del mismo grupo de 5 (o del riel) de esa etiqueta, así no hace falta que quepan todos juntos.
fn unir_red(k: &Circuito, red: &str) -> Result<(Circuito, Vec<String>), String> {
    let labels: Vec<&Comp> = k
        .componentes
        .iter()
        .filter(|x| x.tipo == Tipo::Etiqueta && x.red() == red)
        .collect();
    let pins: Vec<Hole> = labels
        .iter()
        .filter_map(|x| x.holes().ok().map(|h| h[0]))
        .collect();
    if pins.len() < 2 || pins.len() != labels.len() {
        return Err(format!("«{red}» tiene una sola etiqueta"));
    }
    let color = labels[0]
        .color
        .clone()
        .unwrap_or_else(|| draw::net_hex(red));
    let carpetas: Vec<Option<String>> = labels.iter().map(|x| x.carpeta.clone()).collect();
    let mut n = k.clone();
    n.componentes
        .retain(|x| !(x.tipo == Tipo::Etiqueta && x.red() == red));
    let mut ocupados: HashSet<Hole> = n
        .componentes
        .iter()
        .filter_map(|x| x.holes().ok())
        .flatten()
        .collect();
    let todos = Hole::all(n.tableros, n.columnas);
    let d = |a: Hole, b: Hole| {
        let (p, q) = (route::pos(k, a), route::pos(k, b));
        (p.0 - q.0).abs() + (p.1 - q.1).abs()
    };
    let mut hecho = vec![false; pins.len()];
    hecho[0] = true;
    let mut libres_propios: HashSet<Hole> = [pins[0]].into(); // el orificio de la etiqueta ya conectada sirve una vez
    let mut ids = Vec::new();
    for _ in 1..pins.len() {
        // la etiqueta pendiente más cercana a alguna ya conectada que tenga un orificio libre en su grupo
        let mut mejor: Option<(f32, usize, Hole)> = None;
        for (j, _) in hecho.iter().enumerate().filter(|(_, h)| !**h) {
            for (i, _) in hecho.iter().enumerate().filter(|(_, h)| **h) {
                let from = if libres_propios.contains(&pins[i]) {
                    Some(pins[i])
                } else {
                    todos
                        .iter()
                        .copied()
                        .filter(|x| {
                            x.strip() == pins[i].strip()
                                && !ocupados.contains(x)
                                && !pins.contains(x)
                        })
                        .min_by(|a, b| d(*a, pins[j]).total_cmp(&d(*b, pins[j])))
                };
                if let Some(f) = from {
                    let dist = d(f, pins[j]);
                    if mejor.is_none_or(|m| dist < m.0) {
                        mejor = Some((dist, j, f));
                    }
                }
            }
        }
        let Some((_, j, from)) = mejor else {
            return Err(format!("no hay orificios libres para unir «{red}»"));
        };
        libres_propios.remove(&from);
        ocupados.insert(from);
        ocupados.insert(pins[j]);
        hecho[j] = true;
        let mut w = Comp::nuevo(
            &n.next_id(Tipo::Cable.prefix()),
            Tipo::Cable,
            &[&from.name(), &pins[j].name()],
        );
        w.color = Some(color.clone());
        w.carpeta = carpetas[j].clone();
        ids.push(w.id.clone());
        n.componentes.push(w);
    }
    Ok((n, ids))
}

fn save_png(path: &Path, img: &egui::ColorImage) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let [w, h] = img.size;
    let buf: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
    image::save_buffer(path, &buf, w as u32, h as u32, image::ColorType::Rgba8)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cable_etiquetas_ida_y_vuelta() {
        let f = std::env::temp_dir().join("protoboard-conv-no-existe.json");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[0].clone();
        app.recompute();
        let leds_on = |a: &App| a.sim.leds.iter().filter(|l| l.on).count();
        assert_eq!(leds_on(&app), 1);
        app.convertir("W1");
        assert!(app.circ.find("W1").is_none());
        let e = app
            .circ
            .componentes
            .iter()
            .find(|c| c.tipo == Tipo::Etiqueta)
            .unwrap()
            .id
            .clone();
        assert_eq!(
            leds_on(&app),
            1,
            "las etiquetas conectan igual que el cable"
        );
        app.convertir(&e);
        assert!(app
            .circ
            .componentes
            .iter()
            .all(|c| c.tipo != Tipo::Etiqueta));
        assert_eq!(leds_on(&app), 1);
    }

    #[test]
    fn todo_de_golpe_con_color_y_recto() {
        let f = std::env::temp_dir().join("protoboard-conv2-no-existe.json");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[1].clone();
        app.recompute();
        let leds = |a: &App| a.sim.leds.iter().filter(|l| l.on).count();
        let antes = leds(&app);
        let cables: Vec<String> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .map(|c| c.id.clone())
            .collect();
        app.convertir_varios(&cables);
        assert!(app.circ.componentes.iter().all(|c| c.tipo != Tipo::Cable));
        assert_eq!(leds(&app), antes);
        let amarilla = app
            .circ
            .componentes
            .iter()
            .find(|c| c.tipo == Tipo::Etiqueta && c.color.as_deref() == Some("amarillo"));
        assert!(
            amarilla.is_some(),
            "la etiqueta conserva el color del cable"
        );
        let ets: Vec<String> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Etiqueta)
            .map(|c| c.id.clone())
            .collect();
        app.convertir_varios(&ets);
        assert!(app
            .circ
            .componentes
            .iter()
            .all(|c| c.tipo != Tipo::Etiqueta));
        assert_eq!(leds(&app), antes);
        assert!(app
            .circ
            .componentes
            .iter()
            .any(|c| c.tipo == Tipo::Cable && c.color.as_deref() == Some("amarillo")));
        assert!(
            app.circ
                .componentes
                .iter()
                .filter(|c| c.tipo == Tipo::Cable)
                .any(|c| !c.puntos.is_empty()),
            "cables trazados con quiebres"
        );
        if let Ok(p) = std::env::var("VOLCAR_CIRCUITO") {
            guardar(Path::new(&p), &app.circ).unwrap(); // para mirar el resultado con --screenshot
        }
    }

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("protoboard-{name}-{}.json", std::process::id()))
    }

    #[test]
    fn grupos_persisten_al_recargar() {
        let f = temp("grupos");
        let _ = std::fs::remove_file(&f);
        let mut app = App::new(f.clone(), None);
        app.circ = ejemplos()[1].clone();
        app.recompute();
        let cables: Vec<String> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .map(|c| c.id.clone())
            .collect();
        app.mover_a_carpeta(&cables[..3], Some("Cables/rojo".into()));
        drop(app);
        let app = App::new(f.clone(), None);
        for id in &cables[..3] {
            assert_eq!(
                app.circ.find(id).and_then(|c| c.carpeta.as_deref()),
                Some("Cables/rojo")
            );
        }
        assert!(app.circ.carpetas.iter().any(|f| f == "Cables/rojo"));
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn agrupar_por_color_solo_cables() {
        let f = temp("color");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[1].clone();
        app.recompute();
        app.modify(|c| {
            for x in c.componentes.iter_mut().filter(|x| x.tipo == Tipo::Cable) {
                x.carpeta = None;
            }
        });
        let antes: Vec<Option<String>> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo != Tipo::Cable)
            .map(|c| c.carpeta.clone())
            .collect();
        app.agrupar_cables_por_color();
        for c in app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
        {
            let esperado = format!("Cables/{}", c.color_or("rojo"));
            assert_eq!(c.carpeta.as_deref(), Some(esperado.as_str()));
        }
        let despues: Vec<Option<String>> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo != Tipo::Cable)
            .map(|c| c.carpeta.clone())
            .collect();
        assert_eq!(antes, despues);
    }

    #[test]
    fn ocultar_y_enfoque_por_carpeta() {
        let f = temp("foco");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[1].clone();
        app.recompute();
        let cables: Vec<String> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .map(|c| c.id.clone())
            .collect();
        app.mover_a_carpeta(&cables[..2], Some("Cables/rojo".into()));
        app.mover_a_carpeta(&cables[2..4], Some("Cables/azul".into()));
        let rojo = app.circ.find(&cables[0]).unwrap().clone();
        let azul = app.circ.find(&cables[2]).unwrap().clone();
        assert!(App::en_grupo(&rojo, "Cables/rojo"));
        assert!(!App::en_grupo(&rojo, "Cables/azul"));
        assert_eq!(app.ids_de("Cables/rojo").len(), 2);
        let leds = app.sim.leds.iter().filter(|l| l.on).count();
        app.toggle_oculta("Cables/rojo");
        assert!(app.circ.oculta(&rojo));
        assert!(!app.circ.oculta(&azul));
        assert_eq!(app.sim.leds.iter().filter(|l| l.on).count(), leds);
        app.mostrar_todo();
        assert!(!app.circ.oculta(&rojo));
    }

    #[test]
    fn rehacer_restaura_y_se_limpia() {
        let f = temp("redo");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[1].clone();
        app.recompute();
        let antes = app.circ.clone();
        app.mover_a_carpeta(&["W1".into()], Some("Cables/rojo".into()));
        let despues = app.circ.clone();
        assert_ne!(antes, despues);
        app.undo_last();
        assert_eq!(app.circ, antes);
        app.redo_last();
        assert_eq!(app.circ, despues);
        app.modify(|c| c.nombre = "otro".into());
        assert!(app.redo.is_empty());
    }

    #[test]
    fn crear_carpeta_normaliza_nombres() {
        let f = temp("carpeta");
        let mut app = App::new(f.clone(), Some(f));
        app.crear_carpeta("  Cables//rojo  ");
        assert!(app.circ.carpetas.iter().any(|x| x == "Cables/rojo"));
        let n = app.circ.carpetas.len();
        app.crear_carpeta("Cables/rojo");
        assert_eq!(app.circ.carpetas.len(), n, "no duplica carpetas");
        app.crear_carpeta("   ");
        assert_eq!(app.circ.carpetas.len(), n, "ignora nombres vacíos");
    }

    #[test]
    fn reordenar_por_grupos_deja_tramos_rectos() {
        let f = temp("rutas");
        let mut app = App::new(f.clone(), Some(f));
        app.circ = ejemplos()[1].clone();
        app.recompute();
        let cables: Vec<String> = app
            .circ
            .componentes
            .iter()
            .filter(|c| c.tipo == Tipo::Cable)
            .map(|c| c.id.clone())
            .collect();
        let mitad = cables.len() / 2;
        app.mover_a_carpeta(&cables[..mitad], Some("Cables/rojo".into()));
        app.mover_a_carpeta(&cables[mitad..], Some("Cables/azul".into()));
        app.ordenar_grupos(None);
        let mut alguno = false;
        for id in &cables {
            let c = app.circ.find(id).unwrap();
            if c.puntos.is_empty() {
                continue;
            }
            alguno = true;
            let hs = c.holes().unwrap();
            let (a, b) = (route::pos(&app.circ, hs[0]), route::pos(&app.circ, hs[1]));
            let pts: Vec<(f32, f32)> = std::iter::once(a)
                .chain(c.puntos.iter().map(|q| (q[0], q[1])))
                .chain(std::iter::once(b))
                .collect();
            assert!(
                pts.windows(2).all(|w| w[0].0 == w[1].0 || w[0].1 == w[1].1),
                "{id}: {pts:?}"
            );
        }
        assert!(alguno, "ningún cable se pudo rutear");
    }
}
