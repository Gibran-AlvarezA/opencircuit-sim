//! Servidor MCP (Model Context Protocol) por stdio: los agentes de IA arman circuitos
//! editando el archivo compartido, y la ventana del programa se actualiza sola.
use crate::{instr, model::*, sim};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

pub fn run(file: &Path) {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg, file),
            Err(e) => Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":e.to_string()}}),
            ),
        };
        if let Some(r) = resp {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}

pub fn handle(msg: &Value, file: &Path) -> Option<Value> {
    let id = msg.get("id")?.clone(); // sin id = notificación: no se responde
    let method = msg["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => json!({
            "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2024-11-05"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "opencircuit", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Simulador de circuitos en protoboard. Llama primero a ver_guia para conocer el formato. Cada cambio aparece en vivo en la ventana del programa."
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let args = msg["params"].get("arguments").cloned().unwrap_or(json!({}));
            match call(name, &args, file) {
                Ok(t) => json!({"content":[{"type":"text","text":t}],"isError":false}),
                Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
            }
        }
        _ => {
            return Some(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":format!("Método no soportado: {method}")}}),
            )
        }
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn tool(name: &str, desc: &str, props: Value, req: &[&str]) -> Value {
    json!({"name": name, "description": desc, "inputSchema": {"type":"object","properties": props, "required": req}})
}

fn tools() -> Vec<Value> {
    let comp = json!({
        "type": "object",
        "description": "Componente físico o conexión del circuito. Consulta ver_guia para pinouts y reglas.",
        "properties": {
            "id": {"type": "string"},
            "tipo": {
                "type": "string",
                "enum": [
                    "fuente", "resistencia", "led", "chip", "interruptor",
                    "potenciometro", "pulsador", "cable", "display", "etiqueta"
                ]
            },
            "pines": {
                "type": "array",
                "items": {"type": "string"},
                "minItems": 1
            },
            "valor": {"type": "number"},
            "posicion": {"type": "number", "minimum": 0, "maximum": 1},
            "color": {"type": "string"},
            "modelo": {"type": "string"},
            "nombre": {"type": "string"},
            "cerrado": {"type": "boolean"},
            "presionado": {"type": "boolean"},
            "puntos": {
                "type": "array",
                "items": {
                    "type": "array",
                    "items": {"type": "number"},
                    "minItems": 2,
                    "maxItems": 2
                }
            },
            "carpeta": {"type": "string"}
        },
        "required": ["tipo", "pines"]
    });
    vec![
        tool("ver_guia", "Explica la protoboard (nombres de orificios), los componentes, los chips y sus pines, con un ejemplo. Léela antes de armar.", json!({}), &[]),
        tool("ver_circuito", "Devuelve el circuito actual en JSON.", json!({}), &[]),
        tool("definir_circuito", "Reemplaza el circuito entero. Se valida antes de guardar.", json!({"circuito": {"type":"object","description":"{nombre, componentes:[...]} según ver_guia"}}), &["circuito"]),
        tool("agregar_componente", "Añade un componente (si no tiene id se le asigna uno). Rechaza orificios ocupados o inexistentes.", json!({"componente": comp}), &["componente"]),
        tool("eliminar_componente", "Quita un componente por su id.", json!({"id":{"type":"string"}}), &["id"]),
        tool("cambiar_interruptor", "Abre o cierra un interruptor.", json!({"id":{"type":"string"},"cerrado":{"type":"boolean"}}), &["id","cerrado"]),
        tool("simular", "Simula el circuito: estado de LEDs (mA), salidas de los chips, errores, advertencias y tabla de verdad de los interruptores.", json!({}), &[]),
        tool("instrucciones_armado", "Instrucciones paso a paso para armar el circuito físicamente (Markdown).", json!({}), &[]),
        tool("cargar_ejemplo", "Carga un circuito de ejemplo: 1 = LED con resistencia, 2 = compuerta AND, 3 = inversor NOT.", json!({"numero":{"type":"integer","minimum":1,"maximum":3}}), &["numero"]),
        tool("limpiar", "Deja la protoboard vacía.", json!({}), &[]),
    ]
}

fn load(file: &Path) -> Result<Circuito, String> {
    if file.exists() {
        cargar(file)
    } else {
        Ok(Circuito::default())
    }
}

fn save(file: &Path, c: &Circuito, msg: String) -> Result<String, String> {
    let errs = c.validate();
    if !errs.is_empty() {
        return Err(format!(
            "No se guardó: el circuito tiene errores:\n- {}",
            errs.join("\n- ")
        ));
    }
    guardar(file, c)?;
    Ok(format!("{msg}\n\n{}", sim::resumen(c)))
}

pub fn call(name: &str, a: &Value, file: &Path) -> Result<String, String> {
    match name {
        "ver_guia" => Ok(guia()),
        "ver_circuito" => Ok(serde_json::to_string_pretty(&load(file)?).unwrap()),
        "simular" => Ok(sim::resumen(&load(file)?)),
        "instrucciones_armado" => {
            let c = load(file)?;
            let s = sim::simulate(&c);
            let t = sim::tabla(&c);
            Ok(instr::markdown(
                &c.nombre,
                &instr::instrucciones(&c, &s, t.as_ref()),
            ))
        }
        "definir_circuito" => {
            let v = a.get("circuito").cloned().unwrap_or_else(|| a.clone());
            let c = match v {
                Value::String(s) => parse_text(&s)?,
                v => {
                    let mut c: Circuito = serde_json::from_value(v)
                        .map_err(|e| format!("Circuito no válido: {e}. Consulta ver_guia."))?;
                    c.completar_ids();
                    c
                }
            };
            save(
                file,
                &c,
                format!("Circuito guardado ({} componentes).", c.componentes.len()),
            )
        }
        "agregar_componente" => {
            let v = a.get("componente").cloned().unwrap_or_else(|| a.clone());
            let mut comp: Comp = serde_json::from_value(v)
                .map_err(|e| format!("Componente no válido: {e}. Consulta ver_guia."))?;
            let mut c = load(file)?;
            if comp.id.trim().is_empty() {
                comp.id = c.next_id(comp.tipo.prefix());
            }
            let antes = c.validate();
            c.componentes.push(comp.clone());
            let nuevos: Vec<String> = c
                .validate()
                .into_iter()
                .filter(|e| !antes.contains(e))
                .collect();
            if !nuevos.is_empty() {
                return Err(format!(
                    "No se añadió {}:\n- {}",
                    comp.id,
                    nuevos.join("\n- ")
                ));
            }
            guardar(file, &c)?;
            Ok(format!(
                "Añadido {} ({}).\n\n{}",
                comp.id,
                comp.tipo.nombre(),
                sim::resumen(&c)
            ))
        }
        "eliminar_componente" => {
            let id = a["id"].as_str().ok_or("Falta 'id'")?;
            let mut c = load(file)?;
            let n = c.componentes.len();
            c.componentes.retain(|x| x.id != id);
            if c.componentes.len() == n {
                return Err(format!("No existe el componente {id}"));
            }
            guardar(file, &c)?;
            Ok(format!("Eliminado {id}.\n\n{}", sim::resumen(&c)))
        }
        "cambiar_interruptor" => {
            let id = a["id"].as_str().ok_or("Falta 'id'")?;
            let cerrado = a["cerrado"]
                .as_bool()
                .ok_or("Falta 'cerrado' (true/false)")?;
            let mut c = load(file)?;
            let s = c
                .componentes
                .iter_mut()
                .find(|x| x.id == id && x.tipo == Tipo::Interruptor)
                .ok_or(format!("No hay un interruptor con id {id}"))?;
            s.cerrado = cerrado;
            guardar(file, &c)?;
            Ok(format!(
                "{id} ahora está {}.\n\n{}",
                if cerrado { "cerrado" } else { "abierto" },
                sim::resumen(&c)
            ))
        }
        "cargar_ejemplo" => {
            let n = a["numero"].as_u64().unwrap_or(1) as usize;
            let c = ejemplos()
                .get(n.saturating_sub(1))
                .cloned()
                .ok_or("Ejemplos disponibles: 1, 2 y 3")?;
            save(file, &c, format!("Ejemplo «{}» cargado.", c.nombre))
        }
        "limpiar" => save(file, &Circuito::default(), "Protoboard vacía.".into()),
        _ => Err(format!("Herramienta desconocida: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flujo_de_agente() {
        let f = std::env::temp_dir().join(format!("protoboard-mcp-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&f);
        let r = handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}), &f).unwrap();
        assert!(r["result"]["tools"].as_array().unwrap().len() >= 9);
        assert!(handle(
            &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            &f
        )
        .is_none());
        call("cargar_ejemplo", &json!({"numero":1}), &f).unwrap();
        assert!(call("simular", &json!({}), &f)
            .unwrap()
            .contains("ENCENDIDO"));
        let e = call(
            "agregar_componente",
            &json!({"componente":{"tipo":"led","pines":["d9","z1"]}}),
            &f,
        );
        assert!(e.is_err());
        call(
            "agregar_componente",
            &json!({"componente":{"tipo":"led","color":"azul","pines":["e20","e21"]}}),
            &f,
        )
        .unwrap();
        assert!(cargar(&f).unwrap().find("D2").is_some());
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn schema_mcp_declara_todos_los_tipos_de_componente() {
        let listado = tools();
        let agregar = listado
            .iter()
            .find(|tool| tool["name"] == "agregar_componente")
            .expect("debe existir agregar_componente");
        let schema = &agregar["inputSchema"]["properties"]["componente"];
        let tipos = schema["properties"]["tipo"]["enum"]
            .as_array()
            .expect("tipo debe tener enum")
            .iter()
            .map(|tipo| tipo.as_str().unwrap())
            .collect::<Vec<_>>();

        let tipos_esperados = [
            "fuente",
            "resistencia",
            "led",
            "chip",
            "interruptor",
            "potenciometro",
            "pulsador",
            "cable",
            "display",
            "etiqueta",
        ];
        assert_eq!(tipos.len(), tipos_esperados.len());
        for esperado in tipos_esperados {
            assert!(tipos.contains(&esperado), "falta el tipo MCP {esperado}");
        }

        let campos = [
            ("id", "string"),
            ("tipo", "string"),
            ("pines", "array"),
            ("valor", "number"),
            ("posicion", "number"),
            ("color", "string"),
            ("modelo", "string"),
            ("nombre", "string"),
            ("cerrado", "boolean"),
            ("presionado", "boolean"),
            ("puntos", "array"),
            ("carpeta", "string"),
        ];
        assert_eq!(schema["required"], json!(["tipo", "pines"]));
        for (campo, tipo) in campos {
            assert_eq!(
                schema["properties"][campo]["type"], tipo,
                "tipo JSON incorrecto para {campo}"
            );
        }
        assert_eq!(schema["properties"]["pines"]["items"]["type"], "string");
        assert_eq!(schema["properties"]["pines"]["minItems"], 1);
        assert_eq!(schema["properties"]["posicion"]["type"], "number");
        assert_eq!(schema["properties"]["posicion"]["minimum"], 0);
        assert_eq!(schema["properties"]["posicion"]["maximum"], 1);
        assert_eq!(schema["properties"]["puntos"]["type"], "array");
        assert_eq!(schema["properties"]["puntos"]["items"]["type"], "array");
        assert_eq!(
            schema["properties"]["puntos"]["items"]["items"]["type"],
            "number"
        );
        assert_eq!(schema["properties"]["puntos"]["items"]["minItems"], 2);
        assert_eq!(schema["properties"]["puntos"]["items"]["maxItems"], 2);
    }
}
