//! Lista de materiales: piezas agrupadas y exportables.
use crate::model::*;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Line {
    pub grupo: &'static str,
    pub detalle: String,
    pub cantidad: usize,
}

pub fn lineas(c: &Circuito) -> Vec<Line> {
    let mut cuenta: BTreeMap<(&'static str, String), usize> = BTreeMap::new();
    for x in &c.componentes {
        let (grupo, detalle) = match x.tipo {
            Tipo::Chip => {
                let m = x.modelo.clone().unwrap_or_default();
                let d = chip_info(&m)
                    .map(|i| i.modelo.to_string())
                    .unwrap_or_else(|| m);
                ("CI", d)
            }
            Tipo::Resistencia => ("Resistencias", fmt_ohm(x.ohmios())),
            Tipo::Led => ("LEDs", x.color_or("rojo")),
            Tipo::Interruptor => ("Interruptores", "Interruptor".into()),
            Tipo::Potenciometro => ("Potenciómetros", fmt_ohm(x.ohmios())),
            Tipo::Pulsador => ("Pulsadores", "Pulsador momentáneo".into()),
            Tipo::Display => ("Displays", "Display 7 segmentos".into()),
            Tipo::Fuente => ("Fuentes", format!("Fuente {:.1} V", x.voltios())),
            Tipo::Cable => {
                let Ok(hs) = x.holes() else { continue };
                let mm = crate::instr::largo_mm(c, hs[0], hs[1]);
                (
                    "Cables",
                    format!("Cable {} · {} mm", x.color_or("rojo"), mm),
                )
            }
            Tipo::Etiqueta => continue,
        };
        *cuenta.entry((grupo, detalle)).or_default() += 1;
    }
    cuenta
        .into_iter()
        .map(|((grupo, detalle), cantidad)| Line {
            grupo,
            detalle,
            cantidad,
        })
        .collect()
}

pub fn csv(c: &Circuito) -> String {
    let mut s = String::from("grupo,detalle,cantidad\n");
    for l in lineas(c) {
        s.push_str(&format!(
            "{},{},{}\n",
            esc(l.grupo),
            esc(&l.detalle),
            l.cantidad
        ));
    }
    s
}

fn esc(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prac(json: &str) -> Circuito {
        serde_json::from_str(json).unwrap()
    }

    fn cant(c: &Circuito, grupo: &str, detalle: &str) -> usize {
        lineas(c)
            .iter()
            .find(|l| l.grupo == grupo && l.detalle == detalle)
            .map(|l| l.cantidad)
            .unwrap_or(0)
    }

    #[test]
    fn practica5_tiene_las_piezas_esperadas() {
        let c = prac(include_str!("../assets/practica5.json"));
        assert_eq!(cant(&c, "CI", "74HC08"), 6);
        assert_eq!(cant(&c, "CI", "74HC32"), 5);
        assert_eq!(cant(&c, "CI", "74HC04"), 1);
        assert_eq!(cant(&c, "Interruptores", "Interruptor"), 5);
        assert_eq!(cant(&c, "LEDs", "rojo"), 1);
        assert_eq!(cant(&c, "LEDs", "verde"), 1);
        assert_eq!(cant(&c, "Displays", "Display 7 segmentos"), 1);
    }

    #[test]
    fn practica6_cuenta_sus_chips() {
        let r = prac(include_str!(
            "../assets/Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json"
        ));
        assert_eq!(cant(&r, "CI", "74HC08"), 4);
        assert_eq!(cant(&r, "CI", "74HC32"), 3);
        assert_eq!(cant(&r, "CI", "74HC04"), 1);

        let b = prac(include_str!(
            "../assets/Practica 6 - Daniel Gibran Alvarez Armienta - celular.json"
        ));
        assert_eq!(cant(&b, "CI", "74HC08"), 3);
        assert_eq!(cant(&b, "CI", "74HC32"), 5);
        assert_eq!(cant(&b, "CI", "74HC04"), 1);
    }

    #[test]
    fn cables_del_mismo_color_y_largo_se_agrupan() {
        let mut w1 = Comp::nuevo("W1", Tipo::Cable, &["a1", "a10"]);
        w1.color = Some("rojo".into());
        let mut w2 = Comp::nuevo("W2", Tipo::Cable, &["b1", "b10"]);
        w2.color = Some("rojo".into());
        let mut w3 = Comp::nuevo("W3", Tipo::Cable, &["a1", "a20"]);
        w3.color = Some("rojo".into());
        let c = Circuito {
            componentes: vec![w1, w2, w3],
            ..Default::default()
        };
        let todas = lineas(&c);
        let cables: Vec<&Line> = todas.iter().filter(|l| l.grupo == "Cables").collect();
        assert_eq!(cables.len(), 2, "{todas:?}");
        assert_eq!(cables.iter().map(|l| l.cantidad).sum::<usize>(), 3);
        assert_eq!(cables.iter().map(|l| l.cantidad).max(), Some(2));
    }

    #[test]
    fn csv_escapa_y_exporta() {
        let c = prac(include_str!("../assets/practica5.json"));
        let s = csv(&c);
        assert!(s.starts_with("grupo,detalle,cantidad\n"));
        assert!(s.contains("CI,74HC08,6\n"));
        assert!(s.contains("Cables,"));
    }
}
