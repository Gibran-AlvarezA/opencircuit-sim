# OpenCircuit

**Simulador de circuitos sobre protoboard** con componentes físicos, cables, compuertas 74HC, display de 7 segmentos y guía de armado paso a paso. Incluye integración MCP para que agentes de IA armen y editen circuitos.

[![CI](https://github.com/Gibran-AlvarezA/opencircuit-sim/actions/workflows/ci.yml/badge.svg)](https://github.com/Gibran-AlvarezA/opencircuit-sim/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-stable-orange)

> OpenCircuit is a native breadboard circuit simulator with physical components, step-by-step assembly instructions and MCP support for AI agents.

![OpenCircuit](docs/screenshots/protoboard.png)

## Características

- Protoboard de 1 a 3 placas, 30 o 60 columnas, con rieles de alimentación.
- Fuente de 5 V, resistencias, LEDs, interruptores, potenciómetro, pulsador momentáneo, display de 7 segmentos, cables y etiquetas de red.
- Chips DIP-14: 74HC08, 74HC32, 74HC86, 74HC00, 74HC02 y 74HC04 (alias 7408/7432/7486/7400/7402/7404).
- Chips DIP-16: 74138, 74283, 7483, 7448, 74151, 74157 y 7485.
- Simulación nodal: voltajes, corriente de LEDs, display y tabla de verdad; el potenciómetro y el pulsador usan el mismo solver.
- Trazado ortogonal de cables que esquiva piezas y respeta los pasillos.
- Grupos de cables: crear carpetas, mover piezas, ocultar, enfocar, buscar y reordenar con distinta separación.
- Instrucciones de armado paso a paso y exportación a Markdown.
- Verificación eléctrica (ERC): LED sin resistencia, chip sin alimentación, entradas al aire, cortos, salidas en conflicto y display sin común; cada aviso se centra al hacer clic.
- Lista de materiales agrupada, con exportación a CSV.
- Autosave, copias de seguridad y aviso cuando otro programa cambia el archivo compartido.
- Deshacer, rehacer, selección múltiple y atajos de teclado.
- Integración MCP para Claude Desktop, Claude Code, Cursor y otros agentes.
- Práctica 5 incluida: detector 2 de 5 con 3 placas, 12 CI, display, LEDs y 32 combinaciones.
- Práctica 6 incluida: rumbo de navegación (8 CI) y decodificador BCD del celular (9 CI), 3 placas cada uno y 16 combinaciones.
- Práctica 7 incluida: decodificador 74138 con Σm(1,2) y sumador 74283 + 7448 + display, con 8 y 256 combinaciones verificadas.
- Proyecto final incluido: sumador-restador de 4 bits en BCD con conversión BCD↔binario, magnitud, signo y 512 combinaciones verificadas.
- Ejemplos de 74151, 74157, 7485, potenciómetro, pulsador y divisor resistivo.

## Capturas

| Protoboard | Grupos de cables |
|---|---|
| ![Protoboard](docs/screenshots/protoboard.png) | ![Grupos](docs/screenshots/grupos.png) |

| Enfoque de un grupo | Instrucciones de armado |
|---|---|
| ![Enfoque](docs/screenshots/foco.png) | ![Armado](docs/screenshots/armado.png) |

## Requisitos

- Rust stable (probado con la versión estable actual).
- Linux: dependencias de egui/winit (X11, Wayland y OpenGL).
- macOS: `xcode-select --install`.
- Windows: Visual Studio Build Tools con la carga de trabajo de C++.

En Ubuntu/Debian:

```bash
sudo apt install build-essential pkg-config \
  libxkbcommon-dev libwayland-dev libx11-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-x11-dev libgl1-mesa-dev \
  libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev
```

En Fedora:

```bash
sudo dnf install gcc pkg-config libxkbcommon-devel wayland-devel \
  libX11-devel libxcb-devel mesa-libGL-devel \
  libXrandr-devel libXi-devel libXcursor-devel libXinerama-devel
```

## Compilar y ejecutar

```bash
git clone https://github.com/Gibran-AlvarezA/opencircuit-sim.git
cd opencircuit-sim

cargo run --release
```

Abrir directamente la Práctica 5:

```bash
cargo run --release -- --archivo assets/practica5.json
```

Abrir el panel de grupos:

```bash
cargo run --release -- --archivo assets/practica5.json --panel grupos
```

Abrir la Práctica 6 (dos circuitos):

```bash
cargo run --release -- --archivo "assets/Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json"
cargo run --release -- --archivo "assets/Practica 6 - Daniel Gibran Alvarez Armienta - celular.json"
```

Regenerar esos dos archivos (verifica la tabla de verdad antes de escribirlos):

```bash
python3 tools/gen_practica6.py
```

Si no indicas `--archivo`, la app usa `~/CircuitosIA/circuito_actual.json` y lo crea si no existe.

## Uso rápido

1. Elige una pieza en el panel izquierdo y haz clic en los orificios.
2. Para un cable, haz clic en el primer orificio y luego en el segundo.
3. Para un chip, haz clic en la columna del pin 1 (fila e o f).
4. Para el display de 1.2", haz clic en el orificio del pin `g` (arriba a la izquierda): el módulo coloca sus 10 pines en dos filas (g f común a b arriba; e d común c dp abajo).
5. Las etiquetas con el mismo nombre conectan redes entre placas.
6. En la pestaña **Grupos**, crea carpetas como `Cables/rojo`, mueve la selección, oculta o enfoca un grupo, y usa **Reordenar por grupos**.

### Atajos

| Tecla | Acción |
|---|---|
| `Ctrl+Z` | Deshacer |
| `Ctrl+Shift+Z` o `Ctrl+Y` | Rehacer |
| `F` | Centrar la selección o el grupo enfocado |
| `Esc` | Cancelar la colocación o quitar el enfoque |
| `Delete` | Quitar la pieza seleccionada |
| Flechas | Navegar pasos en la pestaña de armado |
| Rueda | Zoom |
| Arrastrar | Mover la vista, una placa o un cable |

## Línea de comandos

| Opción | Descripción |
|---|---|
| `--archivo <ruta>` | Archivo de circuito compartido |
| `--simular <ruta>` | Simula y muestra el estado actual |
| `--instrucciones <ruta>` | Genera las instrucciones de armado en Markdown |
| `--bom <ruta>` | Lista de materiales en CSV por stdout |
| `--guia` | Muestra el formato JSON de circuitos |
| `--screenshot <png>` | Guarda una captura y sale |
| `--paso <n>` | Abre en el paso n de las instrucciones |
| `--pestana protoboard\|armado\|ia` | Pestaña inicial |
| `--panel piezas\|grupos` | Panel lateral inicial |
| `--grupo <carpeta>` | Enfoca una carpeta al abrir |
| `--solo` | Muestra solo el grupo enfocado |
| `--mcp` | Servidor MCP por stdio |
| `--help` | Ayuda |

## Práctica 5

El archivo `assets/practica5.json` contiene un detector de 2 de 5 hecho con lógica TTL/CMOS de 5 V:

- 3 protoboards de 60 columnas.
- 6 × 74HC08, 5 × 74HC32 y 1 × 74HC04.
- 5 interruptores, 2 LEDs y un display de 7 segmentos.
- 32 combinaciones de entrada verificadas.

```bash
# Simulación del estado actual
cargo run --release -- --simular assets/practica5.json

# Instrucciones de armado
cargo run --release -- --instrucciones assets/practica5.json

# Ver la guía paso a paso en la app
cargo run --release -- --archivo assets/practica5.json --pestana armado
```

## Práctica 6

Dos circuitos con lógica de 5 V sobre 3 protoboards de 60 columnas cada uno:

| Archivo | Circuito | CI |
|---|---|---|
| `assets/Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json` | Rumbo de navegación (A=N, B=S, C=E, D=O) | 4 × 74HC08, 3 × 74HC32, 1 × 74HC04 |
| `assets/Practica 6 - Daniel Gibran Alvarez Armienta - celular.json` | Decodificador BCD 6864247720 | 3 × 74HC08, 5 × 74HC32, 1 × 74HC04 |

Los ocho rumbos válidos y los diez dígitos están verificados en `assets/checklist_rumbo.csv` y `assets/checklist_celular.csv`; las combinaciones inválidas del rumbo encienden a, d, e, f, g (la app muestra «E») y las del BCD quedan como no importa.

```bash
# Simulación del estado actual (incluye la tabla de verdad de 16 filas)
cargo run --release -- --simular "assets/Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json"
cargo run --release -- --simular "assets/Practica 6 - Daniel Gibran Alvarez Armienta - celular.json"

# Instrucciones de armado
cargo run --release -- --instrucciones "assets/Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json"

# Regenerar los JSON (comprueba la tabla antes de escribir)
python3 tools/gen_practica6.py
```

## Práctica 7

Dos circuitos con CI MSI sobre 3 protoboards de 60 columnas:

| Archivo | Circuito | CI |
|---|---|---|
| `assets/Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json` | 74138 con F = Σm(1,2) = A'·(B ⊕ C) | 74138 + 74HC00 |
| `assets/Practica 7 - Daniel Gibran Alvarez Armienta - sumador.json` | Sumador de 4 bits, display BCD y acarreo | 74283 + 7448 + 74HC00 |

El decodificador enciende el LED solo en m=1 y m=2. El sumador muestra 0-9; 9+9=18 deja S=2, C4=1 y enciende el LED de acarreo. Las sumas 10-15 y 26-30 quedan fuera de BCD y se documentan como tales.

```bash
cargo run --release -- --simular "assets/Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json"
cargo run --release -- --simular "assets/Practica 7 - Daniel Gibran Alvarez Armienta - sumador.json"
cargo run --release -- --instrucciones "assets/Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json"
python3 tools/gen_practica7.py
```

## Proyecto Final

Sumador-restador de números de 4 bits con entradas y salidas BCD, en 3 protoboards de 60 columnas:

- 5 × 74283 (BCD→binario, suma/resta, magnitud y corrección).
- 3 × 74HC86, 4 × 74HC08, 2 × 74HC32 y 1 × 74HC04.
- 2 × 7448 con displays de 7 segmentos y LED de signo.
- 11 interruptores DIP: 5 por número y 1 para suma/resta.
- 512 combinaciones válidas verificadas (16 valores × 16 valores × 2 operaciones).

```bash
cargo run --release -- --simular "assets/Proyecto Final - Daniel Gibran Alvarez Armienta y Jose Salvador Arreguin Verdugo.json"
cargo run --release -- --instrucciones "assets/Proyecto Final - Daniel Gibran Alvarez Armienta y Jose Salvador Arreguin Verdugo.json"
python3 tools/gen_proyecto_final.py
```

![Proyecto final](docs/screenshots/proyecto_final.png)

Ejemplos de los CI restantes y del modo analógico:

```bash
cargo run --release -- --archivo assets/ejemplo_74151.json
cargo run --release -- --archivo assets/ejemplo_74157.json
cargo run --release -- --archivo assets/ejemplo_7485.json
cargo run --release -- --archivo assets/ejemplo_potenciometro.json
cargo run --release -- --archivo assets/ejemplo_pulsador.json
cargo run --release -- --archivo assets/ejemplo_analogico.json
```

| Decodificador 74138 | Sumador 74283 + 7448 |
|---|---|
| ![P7 decodificador](docs/screenshots/practica7_decoder.png) | ![P7 sumador](docs/screenshots/practica7_sumador.png) |

## Formato de circuito

Un circuito es un JSON con `nombre`, `columnas`, `tableros` y `componentes`. Para ver la guía completa, incluyendo orificios, rieles, chips y etiquetas:

```bash
cargo run --release -- --guia
```

Ejemplo mínimo:

```json
{
  "nombre": "LED con resistencia",
  "columnas": 30,
  "tableros": 1,
  "componentes": [
    {"id": "F1", "tipo": "fuente", "valor": 5, "pines": ["t+1", "t-1"]},
    {"id": "W1", "tipo": "cable", "color": "rojo", "pines": ["t+5", "a5"]},
    {"id": "R1", "tipo": "resistencia", "valor": 330, "pines": ["c5", "c9"]},
    {"id": "D1", "tipo": "led", "color": "rojo", "pines": ["d9", "d10"]},
    {"id": "W2", "tipo": "cable", "color": "negro", "pines": ["a10", "t-10"]}
  ]
}
```

Las piezas pueden agruparse con `"carpeta": "Cables/rojo"`. La app guarda los grupos y las carpetas ocultas en el mismo JSON.

## Agentes de IA y MCP

Compila el binario y úsalo como servidor MCP:

```bash
cargo build --release
./target/release/opencircuit --mcp --archivo ./assets/practica5.json
```

En Claude Code:

```bash
claude mcp add opencircuit -- \
  "$(pwd)/target/release/opencircuit" \
  --mcp --archivo "$(pwd)/assets/practica5.json"
```

En Windows PowerShell:

```powershell
claude mcp add opencircuit -- `
  "$PWD\target\release\opencircuit.exe" `
  --mcp --archivo "$PWD\assets\practica5.json"
```

En Claude Desktop, Cursor u otro cliente MCP:

```json
{
  "mcpServers": {
    "opencircuit": {
      "command": "/ruta/a/opencircuit",
      "args": ["--mcp", "--archivo", "/ruta/a/circuito.json"]
    }
  }
}
```

El agente puede consultar la guía, leer el circuito, añadir y quitar piezas, cambiar interruptores, simular y pedir las instrucciones. Cada cambio se guarda en el archivo compartido y la ventana abierta se actualiza sola.

## Pruebas y calidad

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

La suite incluye simulación de compuertas, displays, etiquetas, conversión cables/etiquetas, grupos, rehacer, rutas ortogonales, potenciómetro, pulsador, flujo MCP y validación de su schema, los 32 casos de la Práctica 5, los 16 casos de cada circuito de la Práctica 6, los casos de la Práctica 7 y las 512 combinaciones del proyecto final.

## Estructura del proyecto

```text
src/main.rs    interfaz, herramientas, paneles, grupos y atajos
src/model.rs   circuito, componentes, validación y formato JSON
src/sim.rs     simulación nodal, chips, LEDs y tabla de verdad
src/draw.rs    dibujo físico de protoboard, piezas y cables
src/route.rs   trazado ortogonal de cables con A*
src/instr.rs   instrucciones de armado paso a paso
src/mcp.rs     servidor MCP para agentes
assets/        circuitos y datos de ejemplo
tools/         generadores reproducibles de las Prácticas 5, 6, 7 y proyecto final
docs/          resumen del proceso y capturas
```

Más contexto sobre cómo se construyó: [docs/craft.md](docs/craft.md).

## Limitaciones

- Aplicación nativa de escritorio; no hay versión web.
- Solo tema oscuro.
- El ruteo de cables es heurístico: separa y esquiva piezas, pero no empaqueta haces perfectos.
- Los binarios de macOS y Windows no están firmados; el sistema puede mostrar un aviso.
- El release automático incluye macOS Intel; en Apple Silicon se puede ejecutar con Rosetta o compilar desde fuente.
- La simulación está enfocada en lógica digital a 5 V y no es un simulador SPICE: no modela transitorios, AC ni componentes analógicos continuos. El potenciómetro sí se modela como resistencia ajustable y el pulsador como un interruptor momentáneo dentro del solver nodal.

## Contribuir

Las issues y pull requests son bienvenidas. Antes de enviar cambios:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Mantén los cambios enfocados y añade una prueba cuando toques simulación, ruteo o persistencia.

## Licencia

MIT. Consulta [LICENSE](LICENSE).

---

## English summary

OpenCircuit is a native Rust/egui breadboard simulator. It draws physical components (74HC chips, LEDs, resistors, switches, 7-segment displays, wires and net labels), simulates the circuit, routes wires orthogonally, groups wires into folders, and generates step-by-step assembly instructions. It also exposes an MCP server so AI agents can build and edit circuits. The included Practice 5 example uses 3 breadboards and 12 logic chips.

Build with `cargo run --release`, run the tests with `cargo test`, and see `--guia` for the JSON format.
