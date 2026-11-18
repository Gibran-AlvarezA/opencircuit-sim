//! Guía de armado físico paso a paso, generada a partir del circuito.
use crate::model::*;
use crate::sim::{Sim, Tabla};
use std::collections::BTreeMap;

pub struct Paso {
    pub titulo: String,
    pub texto: Vec<String>,
    /// Componentes que se colocan en este paso (se resaltan en la protoboard).
    pub ids: Vec<String>,
}

const BANDA: [&str; 10] = [
    "negro", "marrón", "rojo", "naranja", "amarillo", "verde", "azul", "violeta", "gris", "blanco",
];

/// Código de colores de 4 bandas (la última, dorada, es la tolerancia del 5 %).
pub fn bandas(ohm: f64) -> Vec<&'static str> {
    let ohm = ohm.max(1.0);
    let mut e = ohm.log10().floor() as i32 - 1;
    let mut d = (ohm / 10f64.powi(e)).round() as i64;
    if d >= 100 {
        d /= 10;
        e += 1;
    }
    let mult = if e < 0 {
        "dorado"
    } else {
        BANDA[e.min(9) as usize]
    };
    vec![
        BANDA[(d / 10) as usize],
        BANDA[(d % 10) as usize],
        mult,
        "dorado",
    ]
}

fn mitad(h: Hole) -> &'static str {
    match h {
        Hole::Main { row, .. } if row < 5 => "mitad superior, filas a-e",
        Hole::Main { .. } => "mitad inferior, filas f-j",
        _ => "riel",
    }
}

fn donde(h: Hole) -> String {
    format!("{} ({})", h.name(), h.describe())
}

fn largo_mm(c: &Circuito, a: Hole, b: Hole) -> i32 {
    let ((x1, y1), (x2, y2)) = (crate::route::pos(c, a), crate::route::pos(c, b));
    let mm = ((x1 - x2).hypot(y1 - y2) * 2.54 + 10.0) as i32;
    (mm + 4) / 5 * 5
}

fn proposito(a: Hole, b: Hole) -> String {
    match (a, b) {
        (Hole::Rail { plus: p1, .. }, Hole::Rail { plus: p2, .. }) => {
            if p1 == p2 {
                format!("Sirve para unir los rieles {} de arriba y de abajo, así los dos tienen alimentación.", if p1 { "positivos (+)" } else { "negativos (-)" })
            } else {
                "¡Atención! Este cable une un riel + con uno -: eso es un cortocircuito.".into()
            }
        }
        (Hole::Rail { plus, .. }, m @ Hole::Main { .. })
        | (m @ Hole::Main { .. }, Hole::Rail { plus, .. }) => format!(
            "Sirve para llevar el {} de la alimentación a la columna {} ({}).",
            if plus { "positivo (+)" } else { "negativo (-)" },
            m.col(),
            mitad(m)
        ),
        _ => format!(
            "Sirve para conectar la columna {} ({}) con la columna {} ({}).",
            a.col(),
            mitad(a),
            b.col(),
            mitad(b)
        ),
    }
}

fn toca_riel(hs: &[Hole]) -> bool {
    hs.iter().any(|h| matches!(h, Hole::Rail { .. }))
}

pub fn instrucciones(c: &Circuito, sim: &Sim, tabla: Option<&Tabla>) -> Vec<Paso> {
    let items: Vec<(&Comp, Vec<Hole>)> = c
        .componentes
        .iter()
        .filter_map(|x| x.holes().ok().map(|h| (x, h)))
        .collect();
    let de = |t: Tipo| items.iter().filter(move |(x, _)| x.tipo == t);
    let mut pasos = Vec::new();

    // 1. Materiales
    let mut mat = vec![format!(
        "{} {} de {} columnas o más, con rieles de alimentación arriba y abajo.",
        c.tableros,
        if c.tableros == 1 {
            "protoboard"
        } else {
            "protoboards"
        },
        c.columnas
    )];
    if c.tableros > 1 {
        mat.push(format!(
            "Vas a usar {} placas. Colócalas como en la vista previa; cada una lleva su rótulo PLACA 1, 2 o 3. Todas comparten el +5 V y el GND mediante cables entre sus rieles.",
            c.tableros
        ));
    }
    for (x, _) in de(Tipo::Fuente) {
        mat.push(format!("1 fuente de {:.1} V de corriente continua ({}) con un cable rojo (+) y uno negro (-). Sirve un módulo de alimentación para protoboard, un cargador USB de 5 V con cables o un portapilas.", x.voltios(), x.id));
    }
    let mut cuenta: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut add = |k: String, id: &str| {
        let e = cuenta.entry(k).or_default();
        e.0 += 1;
        e.1.push(id.to_string());
    };
    for (x, _) in de(Tipo::Chip) {
        let m = x.modelo.clone().unwrap_or_default();
        let n = chip_info(&m).map(|i| i.nombre).unwrap_or("");
        add(
            format!("chip {m} ({n}, encapsulado DIP de 14 patas)"),
            &x.id,
        );
    }
    for (x, _) in de(Tipo::Resistencia) {
        add(
            format!(
                "resistencia de {} (bandas {})",
                fmt_ohm(x.ohmios()),
                bandas(x.ohmios()).join(", ")
            ),
            &x.id,
        );
    }
    for (x, _) in de(Tipo::Led) {
        add(
            format!(
                "LED {} de 5 mm",
                crate::draw::nombre_color(&x.color_or("rojo"))
            ),
            &x.id,
        );
    }
    for (x, _) in de(Tipo::Interruptor) {
        add(
            "interruptor deslizante o pulsador de 2 patas para protoboard".into(),
            &x.id,
        );
    }
    for (x, _) in de(Tipo::Display) {
        let m = if x.pines.len() != 8 {
            "display de 7 segmentos de 1.2 pulgadas de cátodo común (10 pines: g, f, común, a, b arriba; e, d, común, c, dp abajo)"
        } else {
            "display de 7 segmentos de cátodo común (8 terminales: a, b, c, d, e, f, g y común)"
        };
        add(m.into(), &x.id);
    }
    for (x, h) in de(Tipo::Cable) {
        add(
            format!(
                "cable {} de unos {} mm",
                crate::draw::nombre_color(&x.color_or("rojo")),
                largo_mm(c, h[0], h[1])
            ),
            &x.id,
        );
    }
    for (k, (n, ids)) in &cuenta {
        mat.push(format!("{n} × {k}  [{}]", ids.join(", ")));
    }
    let n_etiquetas = c
        .componentes
        .iter()
        .filter(|x| x.tipo == Tipo::Etiqueta)
        .count();
    if n_etiquetas > 0 {
        mat.push(format!(
            "{n_etiquetas} etiquetas de red (papel, cinta o tubo termorretráctil) para marcar los cables con nombres como A, p1, S2 o GND."
        ));
    }
    mat.push("Herramientas: pinzas de punta fina, cortador/pelador de cable y, si tienes, un multímetro.".into());
    mat.push("Entre corchetes está el nombre de cada pieza en el simulador.".into());
    pasos.push(Paso {
        titulo: "Reúne los materiales".into(),
        texto: mat,
        ids: vec![],
    });

    // 2. La protoboard
    pasos.push(Paso {
        titulo: "Conoce y orienta tus placas".into(),
        texto: vec![
            "Pon las placas horizontales frente a ti, como se ven en la vista previa: cada una lleva su rótulo PLACA 1, 2 o 3, la columna 1 a la izquierda y la fila a arriba (los números y letras vienen impresos en la placa).".into(),
            "Cada columna tiene dos grupos de 5 orificios conectados por dentro: a-e (arriba) y f-j (abajo). Lo que pongas en el mismo grupo queda conectado.".into(),
            "El canal del centro separa las dos mitades: entre la fila e y la fila f no hay conexión.".into(),
            "Arriba y abajo están los rieles de alimentación: línea roja = positivo (+), línea azul = negativo (-). Todos los orificios de un riel están conectados a lo largo.".into(),
            "Ojo: en algunas protoboards los rieles están cortados a la mitad (se ve un hueco en la línea roja/azul). Si es tu caso, une las dos mitades con un cable corto.".into(),
            "En el simulador los orificios se nombran por fila y columna (por ejemplo e12) y los rieles como t+5 (riel + de arriba, posición 5) o b-3 (riel - de abajo). Para la placa 2 o 3 se añade @2 o @3: a12@2, t+5@3.".into(),
            "Trabaja siempre con la fuente DESCONECTADA mientras armas.".into(),
        ],
        ids: vec![],
    });

    // 3. Chips
    for (x, h) in de(Tipo::Chip) {
        let m = x.modelo.clone().unwrap_or_default();
        let Some(info) = chip_info(&m) else { continue };
        let col = h[0].col();
        let board = h[0].board();
        let placa = if c.tableros > 1 {
            format!(" en la placa {board}")
        } else {
            String::new()
        };
        pasos.push(Paso {
            titulo: format!("Coloca el chip {} ({}){}", x.id, m, placa),
            texto: vec![
                format!("Toma el chip {m}: tiene {}.", info.nombre),
                "Busca la muesca (media luna) o el puntito en un extremo: marca el lado del pin 1.".into(),
                "Orienta el chip con la muesca hacia la IZQUIERDA (hacia la columna 1).".into(),
                format!("Colócalo encima del canal central{}: las 7 patas de abajo (pines 1 a 7) van en la fila f, columnas {} a {}; las 7 de arriba (pines 14 a 8) en la fila e, columnas {} a {}.", placa, col, col + 6, col, col + 6),
                format!("Comprueba: el pin 1 queda en {} y el pin 14 en {}.", h[0].name(), h[13].name()),
                "Alinea todas las patas antes de empujar y presiona de forma pareja con los dos pulgares hasta que entre del todo. Si las patas están muy abiertas, apóyalas de lado sobre la mesa y dóblalas un poco hacia dentro.".into(),
                format!("Patillaje: pin 14 = VCC (+), pin 7 = GND (-); {}.", info.pinout()),
                if info.gate == Gate::Not {
                    "Ojo con el 7404: las compuertas 4, 5 y 6 tienen la SALIDA en los pines 8, 10 y 12 (la entrada está en 9, 11 y 13). En las compuertas 1 a 3 la entrada es el pin impar y la salida el par. Guíate por los números impresos."
                } else {
                    "Los números impresos marcan el pin 1 en la esquina de la muesca; los pares entrada-salida son los del patillaje."
                }.into(),
                "Estos chips son sensibles a la electricidad estática: toca algo metálico antes de manipularlos.".into(),
            ],
            ids: vec![x.id.clone()],
        });
    }

    // 4. Display
    for (x, h) in de(Tipo::Display) {
        let mut texto = if h.len() >= 10 {
            vec![
                "Toma el display de 1.2 pulgadas de CÁTODO COMÚN: 10 pines en dos filas. Arriba van g, f, común, a, b; abajo van e, d, común, c, dp (el punto decimal). Si el tuyo fuera de ánodo común no funcionaría directamente con estas salidas: necesitarías invertir las señales.".into(),
                "Oriéntalo como en la vista previa: el pin g arriba a la izquierda y el punto decimal abajo a la derecha.".into(),
                "Los dos pines «común» están unidos por dentro: conecta al menos uno a GND (mejor los dos). Si no usas el punto decimal, deja dp sin conectar.".into(),
            ]
        } else {
            vec![
                "Toma el display de 7 segmentos de CÁTODO COMÚN. Si el tuyo es de ánodo común, no funcionará directamente con estas salidas: necesitarías invertir las señales.".into(),
                "Identifica los 8 terminales: a, b, c, d, e, f, g y común. En muchos módulos vienen marcados; si no, usa la hoja de datos del modelo.".into(),
                "El orden del simulador es a, b, c, d, e, f, g, común. El común va a GND. Si tu display tiene dos pines de común, conecta los dos a GND.".into(),
            ]
        };
        for (i, hh) in h.iter().enumerate() {
            texto.push(format!(
                "Inserta el terminal {} en {}.",
                x.pin_label(i),
                donde(*hh)
            ));
        }
        texto.push(
            "No fuerces las patas: si el paso no coincide, revisa el modelo antes de doblarlas."
                .into(),
        );
        pasos.push(Paso {
            titulo: format!("Coloca el display {} (7 segmentos)", x.id),
            texto,
            ids: vec![x.id.clone()],
        });
    }

    // 5-8. Cables de alimentación, resistencias, LEDs, interruptores, cables de señal
    let cable = |x: &Comp, h: &[Hole], tit: &str| Paso {
        titulo: format!(
            "{} {} ({})",
            tit,
            x.id,
            crate::draw::nombre_color(&x.color_or("rojo"))
        ),
        texto: vec![
            format!(
                "Corta un cable {} de unos {} mm (o usa un cable puente de ese color).",
                crate::draw::nombre_color(&x.color_or("rojo")),
                largo_mm(c, h[0], h[1])
            ),
            "Si no es un cable puente, pela unos 7 mm de aislante en cada punta.".into(),
            format!("Conecta un extremo en {}.", donde(h[0])),
            format!("Conecta el otro extremo en {}.", donde(h[1])),
            proposito(h[0], h[1]),
            "Que el cable vaya pegado a la placa: así no se engancha ni se suelta.".into(),
        ],
        ids: vec![x.id.clone()],
    };
    for (x, h) in de(Tipo::Cable).filter(|(_, h)| toca_riel(h)) {
        pasos.push(cable(x, h, "Cable de alimentación"));
    }
    for (x, h) in de(Tipo::Resistencia) {
        pasos.push(Paso {
            titulo: format!("Coloca la resistencia {} ({})", x.id, fmt_ohm(x.ohmios())),
            texto: vec![
                format!(
                    "Toma la resistencia de {}: sus bandas de color son {}.",
                    fmt_ohm(x.ohmios()),
                    bandas(x.ohmios()).join(", ")
                ),
                "Las resistencias no tienen polaridad: da igual hacia qué lado la pongas.".into(),
                "Dobla las dos patas en ángulo recto, cerca del cuerpo.".into(),
                format!("Inserta una pata en {}.", donde(h[0])),
                format!("Inserta la otra pata en {}.", donde(h[1])),
                "Si las patas sobresalen mucho, córtalas para que no toquen otras piezas.".into(),
            ],
            ids: vec![x.id.clone()],
        });
    }
    for (x, h) in de(Tipo::Led) {
        pasos.push(Paso {
            titulo: format!("Coloca el LED {} ({})", x.id, crate::draw::nombre_color(&x.color_or("rojo"))),
            texto: vec![
                format!("Toma el LED {}.", crate::draw::nombre_color(&x.color_or("rojo"))),
                "Identifica sus patas: la LARGA es el ánodo (+) y la CORTA es el cátodo (-). El cátodo también está del lado plano del borde del LED.".into(),
                format!("Inserta la pata LARGA (ánodo, +) en {}.", donde(h[0])),
                format!("Inserta la pata CORTA (cátodo, -) en {}.", donde(h[1])),
                "Si lo pones al revés no se rompe, pero no encenderá.".into(),
            ],
            ids: vec![x.id.clone()],
        });
    }
    for (x, h) in de(Tipo::Interruptor) {
        pasos.push(Paso {
            titulo: format!("Coloca el interruptor {}", x.id),
            texto: vec![
                format!("Toma el interruptor {}.", x.id),
                format!("Inserta una pata en {}.", donde(h[0])),
                format!("Inserta la otra pata en {}.", donde(h[1])),
                format!("Déjalo {} como en el simulador. Cerrado (ON) deja pasar la corriente; abierto (OFF) la corta.", if x.cerrado { "CERRADO (ON)" } else { "ABIERTO (OFF)" }),
                "Si es un pulsador de 4 patas, colócalo sobre el canal central y usa dos patas del mismo lado.".into(),
            ],
            ids: vec![x.id.clone()],
        });
    }

    // 9. Etiquetas de red (una por nombre, con todos sus puntos)
    let mut grupos: BTreeMap<String, Vec<(&Comp, Hole)>> = BTreeMap::new();
    for (x, h) in de(Tipo::Etiqueta) {
        grupos.entry(x.red()).or_default().push((x, h[0]));
    }
    for (name, list) in grupos {
        let mut texto = vec![format!(
            "Prepara una etiqueta con el nombre «{name}». Todas las etiquetas con el mismo nombre quedan conectadas entre sí, aunque estén en placas distintas."
        )];
        for (x, h) in &list {
            texto.push(format!("Pon «{name}» en {} ({}).", donde(*h), x.id));
        }
        texto.push("En Tinkercad no hay etiquetas: une esos puntos con cables del mismo color y anota el nombre de la red en ambos extremos. En una protoboard real, usa cinta o tubo termorretráctil para marcar el cable.".into());
        pasos.push(Paso {
            titulo: format!("Etiqueta de red «{name}»"),
            texto,
            ids: list.iter().map(|(x, _)| x.id.clone()).collect(),
        });
    }

    // 10. Cables de señal (los que no tocan rieles)
    for (x, h) in de(Tipo::Cable).filter(|(_, h)| !toca_riel(h)) {
        pasos.push(cable(x, h, "Cable de señal"));
    }

    // 11. Aviso de corriente del display en físico
    if de(Tipo::Display).next().is_some() {
        pasos.push(Paso {
            titulo: "Si lo armas en físico: la corriente del display".into(),
            texto: vec![
                "En Tinkercad y en Digital el display enciende aunque la salida no tenga corriente suficiente: esos simuladores no limitan la corriente de salida.".into(),
                "En una protoboard real, un 74LS o 74HC no puede entregar la corriente de 5 segmentos a la vez (la salida S2 enciende a, d, e, f y g). El display se verá tenue o el chip se calentará.".into(),
                "Solución recomendada por la guía: usa un buffer/driver 74HCT244 (o un transistor por segmento) entre la salida S2 y las resistencias de esos 5 segmentos. El resto de segmentos (b y c, que van con S) sí pueden ir directos.".into(),
                "Si tu display es de ánodo común, no lo conectes directo: necesita inversores o un driver que active a nivel bajo.".into(),
            ],
            ids: vec![],
        });
    }

    // 12. Revisión
    let mut rev = vec![
        "Antes de conectar la alimentación, compara tu protoboard con la imagen del simulador pieza por pieza.".to_string(),
        "Comprueba que cada LED tiene la pata larga hacia el lado positivo.".into(),
        "Comprueba que ningún cable une directamente el riel + con el riel -.".into(),
        "Revisa que las patas de las resistencias no se toquen entre sí ni con otras piezas.".into(),
        "Comprueba que todas las etiquetas con el mismo nombre estén puestas y que no haya dos redes distintas con el mismo nombre.".into(),
    ];
    if de(Tipo::Chip).next().is_some() {
        rev.push("Comprueba que la muesca de cada chip mira a la izquierda y que ninguna pata quedó doblada por debajo.".into());
    }
    if c.tableros > 1 {
        rev.push("Comprueba que los rieles + de todas las placas están unidos entre sí, y los - también. Sin eso, las placas de la derecha no tienen alimentación.".into());
    }
    for e in &sim.errors {
        rev.push(format!("EL SIMULADOR DETECTA UN PROBLEMA: {e}"));
    }
    pasos.push(Paso {
        titulo: "Revisa todo antes de dar corriente".into(),
        texto: rev,
        ids: vec![],
    });

    // 13. Fuente
    for (x, h) in de(Tipo::Fuente) {
        pasos.push(Paso {
            titulo: format!("Conecta la fuente {} ({:.1} V)", x.id, x.voltios()),
            texto: vec![
                "Mantén la fuente APAGADA o desenchufada mientras la conectas.".into(),
                format!("Cable ROJO (+) de la fuente en {}.", donde(h[0])),
                format!("Cable NEGRO (-) de la fuente en {}.", donde(h[1])),
                "Si usas un módulo de alimentación para protoboard, encájalo en los rieles respetando + y - (y ponlo en 5 V).".into(),
                "Enciende solo después de revisar que no hay ningún cortocircuito entre + y -.".into(),
            ],
            ids: vec![x.id.clone()],
        });
    }

    // 14. Prueba
    let mut prueba = vec!["Enciende la fuente.".to_string()];
    for l in &sim.leds {
        prueba.push(format!(
            "Con los interruptores como están ahora, {} debería estar {}.",
            l.id,
            if l.on { "ENCENDIDO" } else { "apagado" }
        ));
    }
    for d in &sim.displays {
        prueba.push(format!(
            "Con los interruptores como están ahora, {} debería mostrar «{}».",
            d.id, d.text
        ));
    }
    if let Some(t) = tabla {
        prueba.push("Prueba todas las combinaciones de interruptores:".into());
        for (ri, (st, ls)) in t.filas.iter().enumerate() {
            let a: Vec<String> =
                t.sw.iter()
                    .zip(st)
                    .map(|(s, v)| format!("{} {}", s, if *v { "cerrado" } else { "abierto" }))
                    .collect();
            let mut b: Vec<String> = t
                .leds
                .iter()
                .zip(ls)
                .map(|(l, v)| format!("{} {}", l, if *v { "encendido" } else { "apagado" }))
                .collect();
            for (d, ch) in t.displays.iter().zip(&t.disp_filas[ri]) {
                b.push(format!("{d} muestra «{ch}»"));
            }
            prueba.push(format!("   {}  ->  {}", a.join(", "), b.join(", ")));
        }
    }
    prueba.push("Si algo se calienta, huele a quemado o un LED brilla muchísimo y se apaga, desconecta la alimentación de inmediato.".into());
    pasos.push(Paso {
        titulo: "Enciende y prueba".into(),
        texto: prueba,
        ids: vec![],
    });

    // 15. Problemas
    let mut prob: Vec<String> = sim
        .warnings
        .iter()
        .map(|w| format!("Aviso del simulador: {w}"))
        .collect();
    prob.extend([
        "Un LED no enciende: revisa su polaridad y que su pata y la de la resistencia estén en la misma columna (mismo grupo de 5).".to_string(),
        "Nada funciona: comprueba que la fuente da voltaje y que los rieles no están cortados a la mitad.".into(),
        "El chip no responde: mide con el multímetro entre el pin 14 y el pin 7; deben verse unos 5 V.".into(),
        "Una pieza va floja: sácala y vuelve a meterla hasta el fondo; a veces el orificio está gastado, prueba otro de la misma columna.".into(),
        "Para medir: multímetro en voltaje de continua (V DC), punta negra al riel -, punta roja en el punto a medir; compara con la sonda del simulador (pasa el ratón sobre un orificio).".into(),
    ]);
    if de(Tipo::Display).next().is_some() {
        prob.push("El display no enciende: revisa que el común sea cátodo y esté en GND, que cada segmento tenga su resistencia de 470 Ω y que el display no esté al revés.".into());
        prob.push("El display muestra siempre «E»: la señal S nunca llega a 1; revisa P y Tb. Si muestra siempre «1»: S está atascada en 1; revisa U6A y Tb.".into());
    }
    pasos.push(Paso {
        titulo: "Si algo no funciona".into(),
        texto: prob,
        ids: vec![],
    });
    pasos
}

pub fn markdown(nombre: &str, pasos: &[Paso]) -> String {
    let mut o = format!("# Instrucciones de armado: {nombre}\n\n");
    for (i, p) in pasos.iter().enumerate() {
        o += &format!("## Paso {}. {}\n\n", i + 1, p.titulo);
        for t in &p.texto {
            o += &format!("- {t}\n");
        }
        o += "\n";
    }
    o
}

#[cfg(test)]
mod tests {
    #[test]
    fn colores() {
        assert_eq!(
            super::bandas(330.0),
            ["naranja", "naranja", "marrón", "dorado"]
        );
        assert_eq!(
            super::bandas(10000.0),
            ["marrón", "negro", "naranja", "dorado"]
        );
        assert_eq!(
            super::bandas(4.7),
            ["amarillo", "violeta", "dorado", "dorado"]
        );
    }

    #[test]
    fn largo_cable_usa_posiciones() {
        use crate::model::{Circuito, Hole};
        let mut c = Circuito {
            tableros: 2,
            ..Default::default()
        };
        let (a, b) = (Hole::parse("a1").unwrap(), Hole::parse("a1@2").unwrap());
        let separadas = super::largo_mm(&c, a, b);
        // Acercar la placa 2 dos columnas (5,08 mm) acorta el cable en 5 mm tras redondear.
        c.posiciones = vec![[0.0, 0.0], [-2.0, 0.0]];
        assert_eq!(super::largo_mm(&c, a, b), separadas - 5);
    }
}
