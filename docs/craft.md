# Cómo se construyó OpenCircuit

OpenCircuit nació de un pedido concreto: un software que simulase circuitos viéndose físicamente —protoboard, compuertas, LEDs, cables— y que además diera instrucciones paso a paso para armarlos en la vida real, con soporte para agentes de IA.

## Stack

- Rust + `eframe`/`egui` para la interfaz nativa.
- `serde`/`serde_json` para el circuito y el archivo compartido.
- `image` para guardar capturas PNG.
- Un servidor MCP propio en `src/mcp.rs`.

## Decisiones principales

- **Modelo físico**: orificios con filas `a..j`, columnas, rieles `t+`, `t-`, `b+`, `b-` y hasta 3 placas.
- **Componentes**: fuente, resistencia, LED, interruptor, display de 7 segmentos, cable, etiqueta de red y chips 74HC08/32/04.
- **Simulación nodal**: se resuelven redes, voltajes, corriente de LEDs y segmentos del display; la tabla de verdad se calcula bajo demanda.
- **Cableado**: los cables se trazan con A* sobre una rejilla de medio orificio. Esquivan patas, cuerpos y otros cables; los quiebres se pueden arrastrar a mano.
- **Etiquetas**: para circuitos grandes, las redes lógicas se conectan por nombre en lugar de llenar la protoboard de cables. Se pueden convertir cables a etiquetas y viceversa.
- **Práctica 5**: se generó desde un netlist y un checklist de 32 casos con `tools/gen_practica5.py`. Usa 3 placas, 12 CI, 5 interruptores, 2 LEDs y un display.
- **Grupos**: las carpetas del modelo (`Comp.carpeta`) se editan desde la pestaña **Grupos**. Se puede ocultar, enfocar, buscar y reordenar por grupo. Todo se guarda en el mismo JSON.
- **Interfaz**: tema oscuro de laboratorio, panel de piezas/grupos, panel de estado compacto y barra inferior con zoom, selección y grupo activo.

## Verificación

- 23 pruebas automáticas: simulación, compuertas, displays, etiquetas, conversión cables/etiquetas, grupos, rehacer, rutas ortogonales y los 32 casos de la Práctica 5.
- `cargo clippy --all-targets -- -D warnings` sin avisos.
- La Práctica 5 se simula con 228 componentes, 3 placas de 60 columnas y 12 chips.
- Capturas de la interfaz en `docs/screenshots/`.

## Lo que queda

- Tema claro.
- Empaquetado geométrico perfecto de haces de cables.
- Más componentes: potenciómetro, pulsador momentáneo, relevadores.
- Firma y notarización de los binarios de macOS/Windows.
- Versión web o tableta.

## Créditos

Construido con Rust, egui y cariño por las protoboards. El diseño y las decisiones se documentaron con el flujo de Craft Anything.
