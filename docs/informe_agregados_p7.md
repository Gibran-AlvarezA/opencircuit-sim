# Informe de agregados para las Prácticas 6 y 7 en OpenCircuit

**Fecha:** 20 de noviembre de 2026
**Alcance:** app local `opencircuit`, no la web `opencircuits.io`.
**Ruta:** `/home/Daniel/Documentos/Craft Anything/quiero-un-software-que-simule/app`
**Estado:** informe previo a la implementación. No se modificó código de la app.

## 1. Resumen ejecutivo

La Práctica 6 ya está en la app y pasa sus pruebas. En el momento de este análisis la Práctica 7 no existía; el estado final, ya implementado y verificado, está en la sección 11. El bloqueo no era el dibujo de la protoboard: era el modelo de chips, que entonces solo entendía CI DIP de 14 patas con compuertas simples y alimentación fija en los pines 14 y 7. La Práctica 7 necesita tres CI DIP de 16 patas con varias salidas por chip: 74138, 74283/7483 y 7448.

El agregado mínimo es:

1. Generalizar el chip a 14 o 16 patas, con VCC/GND y geometría variables.
2. Añadir funciones MSI por chip, no solo una compuerta repetida.
3. Adaptar simulación, dibujo, ERC, instrucciones, guía y MCP a esa generalización.
4. Añadir pruebas de 8 combinaciones para el 74138 y 256 para el sumador.
5. Recién entonces generar y armar los dos circuitos de la Práctica 7.

No hace falta rehacer la Práctica 6. La línea base al iniciar este análisis era 41 pruebas correctas, incluidas las 16 del rumbo y las 16 del celular.

## 2. Método y evidencia

Se combinaron dos fuentes:

- **Lectura directa del código fuente** de `src/model.rs`, `src/sim.rs`, `src/draw.rs`, `src/instr.rs`, `src/erc.rs`, `src/mcp.rs`, `src/bom.rs` y `src/main.rs`.
- **Craft Anything sobre el binario** `target/release/opencircuit`, para comprobar qué modelos están compilados.

Observado significa que se leyó en el archivo o en la evidencia. Derivado significa que se calculó a partir de eso. Inferido significa que es una conclusión de diseño que todavía debe probarse.

### 2.1 Evidencia del binario

Target: `t_8e0bbefea9bc`
SHA-256: `8e0bbefea9bc439835903fffe9fef94045c97d97d9c35ed6b1b53329cfb3217d`

| Modelo | Strings | Funciones | Imports | Evidencia | Lectura |
|---|---:|---:|---:|---|---|
| 74138 | 0 | 0 | 0 | `ev_7d7ee48cc5d383193a73b9ca552f4fcaaf0aaa6bb85e0c5b4580920b23b61d47` | No aparece en el binario |
| 74283 | 0 | 0 | 0 | `ev_f2a9ae84341fe58025825256a48f33d0e1b7bdbf242981877d65a01cf89054b7` | No aparece en el binario |
| 7448 | 0 strings | 10 nombres | 0 | `ev_cc7526fec9f41e731ee0149efaf49c837d1c31a7a152abdfc8d8f663756e272f` | Los 10 nombres son símbolos LLVM; no hay string del modelo |
| 74HC08 | 5 | 9 | 0 | `ev_c9669f74a1f4f44a0a8063116e94f7994e81c961a7b509187854c6e278235e93` | Sí está compilado y referenciado |

Limitación: una búsqueda sin resultados solo cubre lo buscado. La ausencia en el binario se contrastó con el código fuente, que confirma que `CHIPS` solo contiene 74HC08/32/86/00/02/04 y que no hay ramas para 74138, 74283/7483 ni 7448.

### 2.2 Línea base ejecutada

```text
cd "/home/Daniel/Documentos/Craft Anything/quiero-un-software-que-simule/app"
cargo test
```

Resultado:

```text
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
```

Incluye `practica6_rumbo_16_casos`, `practica6_celular_16_casos` y `practica5_32_casos`.

### 2.3 Frontera con el reporte

Todo lo relacionado con OpenCircuit se documenta solo en este informe, `README.md` y `docs/craft.md`. El reporte y la guía de la Práctica 7 no mencionan la app, Rust, egui, `cargo`, rutas, JSON ni MCP. La verificación final incluye un `grep` sobre ambos archivos y debe devolver vacío.

## 3. Inventario actual

| Área | Estado | Fuente |
|---|---|---|
| Protoboard | 1 a 3 placas, 30 o 60 columnas, rieles y canal central | `src/model.rs`, README |
| Fuente | 5 V por defecto, ajustable en el JSON | `src/model.rs` |
| Pasivos | Resistencia, LED, interruptor, cable y etiqueta de red | `src/model.rs` |
| Display | 7 segmentos, cátodo común, módulo de 10 pines | `src/sim.rs`, `src/model.rs` |
| Chips | 74HC08/32/86/00/02/04, DIP-14, una compuerta por unidad | `src/model.rs:230-267` |
| Alimentación de chip | Pines 14 (VCC) y 7 (GND) fijos | `src/sim.rs:267` |
| Práctica 5 | Detector 2 de 5, 12 CI, 32 casos | `assets/practica5.json` |
| Práctica 6 | Rumbo 8 CI y celular 9 CI, 16 casos cada uno | `assets/Practica 6 - Daniel Gibran Alvarez Armienta - *.json` |
| Práctica 7 | No existe | `assets/`, `tools/`, `src/sim.rs` |
| Generadores | `gen_practica5.py` y `gen_practica6.py` | `tools/` |
| CLI | `--simular`, `--instrucciones`, `--bom`, `--guia`, `--screenshot`, `--mcp` | README |
| Pruebas | 41 correctas | `cargo test` |

## 4. Brechas entre la app y la Práctica 7

| # | Brecha | Por qué la P7 la necesita | Archivos afectados | Riesgo |
|---|---|---|---|---|
| 1 | Todo chip se asume de 14 patas | 74138, 74283/7483 y 7448 son DIP-16 | `model.rs`, `sim.rs`, `draw.rs`, `instr.rs`, `erc.rs`, `main.rs` | Alto: toca casi todo el núcleo |
| 2 | `ChipInfo` describe una sola compuerta repetida | El 74138 tiene 8 salidas y el 7448 tiene 7; no son compuertas iguales | `model.rs`, `sim.rs` | Alto: requiere una función por chip |
| 3 | VCC/GND fijos en 14/7 | En DIP-16 son 16/8 | `sim.rs`, `erc.rs`, `instr.rs` | Medio: error eléctrico silencioso si se olvida |
| 4 | Geometría de 7 columnas | Un DIP-16 ocupa 8 columnas | `model.rs:322-337`, `model.rs:532-539`, `draw.rs:927-929`, `draw.rs:1075-1096`, `main.rs:1527` | Medio: chips encimados o fuera de la placa |
| 5 | La tabla de verdad se corta a 5 interruptores | El sumador tiene 8 | `sim.rs:597-648` | Bajo para la simulación, alto para la verificación visual |
| 6 | Textos de armado dicen “DIP de 14 patas” | La guía debe decir 16 y explicar pines 16/8 | `instr.rs:126`, `instr.rs:226-229`, `main.rs:1804` | Bajo, pero confunde al armar |
| 7 | ERC avisa “pin 14 / pin 7” | Debe usar los pines reales de cada chip | `erc.rs:91-95` | Medio: falsos positivos o avisos engañosos |
| 8 | La guía JSON documenta 14 patas | Los agentes y el MCP copian ese formato | `model.rs:978-1030` | Medio: un agente puede colocar mal el chip |
| 9 | No hay funciones MSI | 74138, 74283/7483 y 7448 no se pueden evaluar | `model.rs`, `sim.rs` | Alto: sin esto no hay P7 |
| 10 | El 7483 tiene patillaje distinto del 74283 | No se pueden intercambiar sin cambiar pines | `model.rs`, `tools/gen_practica7.py` | Medio: un modelo único daría resultados falsos |

## 5. Agregados propuestos

### 5.1 Modelo de chip

Cambiar `ChipInfo` para que describa:

- `pines`: 14 o 16.
- `vcc`: 14 o 16.
- `gnd`: 7 u 8.
- `func`: compuerta simple, decodificador 3 a 8, sumador de 4 bits o decodificador BCD a 7 segmentos.
- Entradas y salidas con nombre, no solo una lista de compuertas.

Las compuertas actuales se conservan como un caso particular de `func`. Los alias 7408/74LS08/74HCT08 deben seguir funcionando.

### 5.2 Geometría

- `chip_pin_hole()`: para 16 patas, pines 1..8 en fila `f`, pines 9..16 en fila `e`, ancho de 8 columnas.
- `Comp::holes()`: generar 14 o 16 orificios según el modelo.
- Límite de columna: `MAX_COLS - 7` para DIP-16, no `MAX_COLS - 6`.
- `draw.rs`: cuerpo de `hs[0]` a `hs[7]` para 16 patas, no a `hs[6]`; el fantasma debe medir 8 columnas.
- `main.rs`: el texto “Circuito integrado 74HC de 14 patas” debe decir el número real.

### 5.3 Simulación

- Leer VCC/GND de `info.vcc`/`info.gnd`, no de `hs[13]`/`hs[6]`.
- Evaluar la función del chip y devolver todas sus salidas en `ChipState.outputs`, que ya es un vector.
- 74138: salidas activas en bajo; habilitación G1=1, /G2A=0, /G2B=0.
- 74283/7483: suma de 4 bits con C0; S1..S4 y C4.
- 7448: 0-9 válidos; 10-15 producen patrones no BCD definidos (no se blankean). Salidas a-g activas en alto; `/LT`, `/BI/RBO` y `/RBI` a VCC.
- Mantener los avisos de alimentación, polaridad y entradas al aire.

La tabla de verdad de la interfaz seguirá limitada a 5 interruptores. Las 256 combinaciones del sumador se verifican en pruebas y en el generador, no imprimiendo 256 filas en la ventana. Si más adelante se quiere una tabla de 8 entradas, se hace con una vista paginada, no en esta fase.

### 5.4 Instrucciones, ERC, MCP y BOM

- `instr.rs`: materiales y pasos con 16 patas; alimentación en 16/8; patillaje del modelo.
- `erc.rs`: VCC/GND desde el modelo; avisar de /G2A, /G2B, G1, /LT, /BI y /RBI sin conectar.
- `model::guia()`: documentar DIP-16 y los modelos nuevos.
- `mcp.rs`: no cambia el formato JSON; el modelo se resuelve con `chip_info`.
- `bom.rs`: ya agrupa por `chip_info`; solo necesita que los modelos existan.

### 5.5 Pruebas

- 74138: 8/8 combinaciones y salidas activas en bajo.
- 74283: 256/256 sumas con C0=0.
- 7483: 256/256 con su patillaje propio.
- 7448: 10 dígitos BCD y 6 patrones no BCD documentados.
- Geometría: un DIP-16 ocupa 16 orificios y no se sale de la placa.
- Regresión: P5 32/32, P6 rumbo 16/16 y celular 16/16.
- `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release`.

## 6. Patillajes y comportamiento esperado

### 6.1 74138

| Pin | Función | Pin | Función |
|---|---|---|---|
| 1 | A | 9 | /Y6 |
| 2 | B | 10 | /Y5 |
| 3 | C | 11 | /Y4 |
| 4 | /G2A | 12 | /Y3 |
| 5 | /G2B | 13 | /Y2 |
| 6 | G1 | 14 | /Y1 |
| 7 | /Y7 | 15 | /Y0 |
| 8 | GND | 16 | VCC |

Habilitado con G1=1, /G2A=0 y /G2B=0. La salida seleccionada vale 0; las demás, 1. Deshabilitado, todas valen 1.

En el datasheet, el pin 1 es A (LSB), el pin 2 es B y el pin 3 es C (MSB). En el reporte, la variable A es la MSB y se cablea al pin 3; la variable C es la LSB y va al pin 1. El orden no se invierte: `m = 4A + 2B + C`.

Para la P7: las tres variables a los switches; `/Y1` (pin 14) y `/Y2` (pin 13) a la primera NAND 7400. Esa NAND entrega `N1` activa en alto. La segunda NAND invierte y **sinkea el cátodo** del LED; el ánodo va a +5 V con 330 Ω. El LED enciende cuando `N1 = 1`, en `001` y `010`. La segunda NAND no es una salida "activa en alto": es un inversor que permite que el TTL hunda la corriente del LED. Función: `F = Σm(1,2) = A'·(B ⊕ C)`.

### 6.2 74283 y 7483

| Pin | 74283 | 7483 |
|---|---|---|
| 1 | S2 | A4 |
| 2 | B2 | S3 |
| 3 | A2 | A3 |
| 4 | S1 | B3 |
| 5 | A1 | VCC |
| 6 | B1 | S2 |
| 7 | C0 | B2 |
| 8 | GND | A2 |
| 9 | C4 | S1 |
| 10 | S4 | A1 |
| 11 | B4 | B1 |
| 12 | A4 | GND |
| 13 | S3 | C0 |
| 14 | A3 | C4 |
| 15 | B3 | S4 |
| 16 | VCC | B4 |

Suma `A + B + C0`. S1 es el bit menos significativo; C4 es el acarreo. Pruebas: 2+3=5, 4+5=9, 9+9=18 (S=0010 y C4=1), 7+8=15 (fuera de BCD), 0+0=0.

### 6.3 7448

| Pin | Función | Pin | Función |
|---|---|---|---|
| 1 | B | 9 | e |
| 2 | C | 10 | d |
| 3 | /LT | 11 | c |
| 4 | /BI | 12 | b |
| 5 | /RBI | 13 | a |
| 6 | D | 14 | g |
| 7 | A | 15 | f |
| 8 | GND | 16 | VCC |

Entradas BCD A, B, C, D. Salidas a-g activas en alto. `/LT`, `/BI/RBO` y `/RBI` a VCC. Los códigos 0-9 son válidos; 10-15 producen patrones no BCD definidos y no se blankean. En la P7, S1..S4 van a A..D y el display es de cátodo común.

El 7448 tiene pull-ups internos de 2 kΩ. Los resistores serie de 330 Ω no son obligatorios; si se usan, reducen el brillo. La corriente de fuente es baja, por eso el display puede verse tenue. No se deben presentar como prohibidos: solo son innecesarios.

### 6.4 74151

| Pin | Función | Pin | Función |
|---|---|---|---|
| 1 | D3 | 9 | C |
| 2 | D2 | 10 | B |
| 3 | D1 | 11 | A |
| 4 | D0 | 12 | D7 |
| 5 | Y | 13 | D6 |
| 6 | W | 14 | D5 |
| 7 | `/G` | 15 | D4 |
| 8 | GND | 16 | VCC |

A es LSB, C es MSB. Con `/G`=0, `CBA` elige D0..D7: `Y = Dn` y `W = /Dn`. Con `/G`=1, Y=0 y W=1. Prueba: 8 selecciones con datos distintos, más `/G`=1.

### 6.5 74157

| Pin | Función | Pin | Función |
|---|---|---|---|
| 1 | S | 9 | 3Y |
| 2 | 1A | 10 | 3B |
| 3 | 1B | 11 | 3A |
| 4 | 1Y | 12 | 4Y |
| 5 | 2A | 13 | 4B |
| 6 | 2B | 14 | 4A |
| 7 | 2Y | 15 | `/E` |
| 8 | GND | 16 | VCC |

`S` es común: S=0 selecciona A; S=1 selecciona B. `/E` activo en bajo: con `/E`=1, todas las salidas Y=0. Prueba: 4 canales con S=0/1 y `/E`=0/1.

### 6.6 7485

| Pin | Función | Pin | Función |
|---|---|---|---|
| 1 | B3 | 9 | B0 |
| 2 | A<B | 10 | A0 |
| 3 | A=B | 11 | B1 |
| 4 | A>B | 12 | A1 |
| 5 | A>B | 13 | A2 |
| 6 | A=B | 14 | B2 |
| 7 | A<B | 15 | A3 |
| 8 | GND | 16 | VCC |

Compara A3..A0 con B3..B0. Para un comparador solo, las entradas de cascada van A<B=0, A=B=1, A>B=0. Si A≠B, las entradas de cascada no importan; si A=B, las salidas siguen a las entradas de cascada. Prueba: 256 comparaciones con cascada A=B y los tres casos de cascada cuando A=B.

### 6.7 Analógico, potenciómetro y pulsador

- `Potenciometro`: 3 pines (A, cursor, B), 10 kΩ por defecto y posición 0..1. Se modela con dos resistencias en el solver nodal existente; el cursor entrega `V = 5·pos` sin carga y el divisor real con carga.
- `Pulsador`: dos pines, normalmente abierto y momentáneo. Presionado une las redes; al soltar vuelve a abrir. No es un interruptor permanente.
- El solver actual ya es nodal DC y soporta resistencias, fuentes fijas, LEDs y salidas de chip. No se crea un solver aparte; se extiende el existente.
- Pruebas: 0 %, 50 % y 100 %; carga de 10 kΩ; pulsador con pull-down; LED con 330 Ω; umbral digital 0.8/2.0 V; P5/P6 sin cambios.

## 7. Matriz de verificación

| Agregado | Prueba | Criterio |
|---|---|---|
| DIP-16 | Colocar un 74138 en `f10` | Ocupa 16 orificios y cabe en la placa |
| Alimentación | Simular con VCC en 16 y GND en 8 | `powered = true`; sin aviso de pin 14/7 |
| 74138 | 8 combinaciones | Solo /Y0..7 seleccionada en 0 |
| 74283 | 256 combinaciones | S y C4 coinciden con la suma |
| 7483 | 256 combinaciones | Igual, con su patillaje |
| 7448 | 0-9 y 10-15 | Segmentos correctos; no BCD documentado |
| 74151 | 8 selecciones y `/G` | Y/W correctos |
| 74157 | 4 canales, S y `/E` | Y correcto; `/E`=1 apaga |
| 7485 | 256 comparaciones y cascada | A<B, A=B, A>B correctos |
| Potenciómetro | 0/50/100 % y carga | 0/2.5/5 V; divisor dentro de 5 % |
| Pulsador | Abierto/cerrado | 0/5 V con pull-down |
| Display | 9+9 | Muestra 2 y C4 enciende el LED |
| ERC | Ambos circuitos | Sin errores ni entradas flotantes |
| P6 | Regresión | 16/16 rumbo y 16/16 celular |
| CLI | `--simular`, `--bom`, `--instrucciones` | Salida correcta en ambos JSON |

## 8. Riesgos y desconocidos

- **Refactor de 14 a 16 patas.** Es el cambio de mayor riesgo. Mitigación: hacerlo aditivo, con pruebas de los chips actuales y de la P6 antes y después.
- **Corriente real del 7448.** El modelo lógico no reproduce el brillo ni los pull-ups internos. La guía debe advertir que no es un 7447 y que el display puede verse tenue.
- **Display físico.** El pinout cambia entre módulos. La guía debe pedir verificar el modelo antes de cablear.
- **7483 contra 74283.** No son intercambiables. Se modelan por separado y el generador elige con `--chip`.
- **Tabla de 256 filas.** No se muestra completa en la interfaz. La verificación exhaustiva va en pruebas y en el generador.
- **Craft Anything analiza el binario, no el código.** Las ausencias se contrastaron con `src/`; el informe no afirma que el código fuente no pueda contener algo que el binario no usa.

## 9. Fuera de alcance

- No se agregan otros CI fuera de 74151, 74157 y 7485.
- No se implementa SPICE transitorio, AC, ruido ni análisis de Fourier; el modelo analógico es DC resistivo.
- No se modelan otros sensores analógicos.
- No se cambian los reportes ni las guías de `Circuitos Digitales`.
- No se sube nada a Classroom.
- No se rehace la Práctica 6.

## 10. Recomendación

Aprobar este informe y pasar a la Fase 1: generalizar el núcleo a DIP-16 y agregar los tres modelos MSI con pruebas. Solo después de que la matriz de la sección 7 esté completa se generan y arman los dos circuitos de la Práctica 7.

El orden propuesto es:

1. Núcleo DIP-16.
2. Modelos 74138, 74283, 7483 y 7448.
3. Modelos 74151, 74157 y 7485.
4. Potenciómetro y pulsador sobre el solver nodal.
5. Pruebas y regresión.
6. Generador, JSON de la P7 y ejemplos.
7. Capturas y documentación.

## Anexo. Evidencia citada

| Evidencia | Qué muestra |
|---|---|
| `ev_7d7ee48cc5d383193a73b9ca552f4fcaaf0aaa6bb85e0c5b4580920b23b61d47` | Búsqueda de `74138` en el binario: 0 strings, 0 funciones, 0 imports |
| `ev_f2a9ae84341fe58025825256a48f33d0e1b7bdbf242981877d65a01cf89054b7` | Búsqueda de `74283`: 0 strings, 0 funciones, 0 imports |
| `ev_cc7526fec9f41e731ee0149efaf49c837d1c31a7a152abdfc8d8f663756e272f` | Búsqueda de `7448`: 0 strings; nombres LLVM sin relación |
| `ev_c9669f74a1f4f44a0a8063116e94f7994e81c961a7b509187854c6e278235e93` | Búsqueda de `74HC08`: 5 strings y 9 funciones; sí está compilado |

## 11. Estado final de la implementación

La Fase 1 y la Fase 2 quedaron implementadas en `src/model.rs`, `src/sim.rs`, `src/draw.rs`, `src/instr.rs`, `src/erc.rs` y `src/main.rs`:

- `ChipInfo` tiene `pines`, `vcc`, `gnd`, `func` y `pin_names`; la geometría, la alimentación y los textos de armado usan esos datos en vez de 14/7 fijos.
- Modelos: 74138, 74283, 7483, 7448, 74151, 74157 y 7485, además de los seis 74HC de 14 patas. El 7483 conserva VCC en 5 y GND en 12.
- `Tipo::Potenciometro` y `Tipo::Pulsador`: el primero son dos conductancias sobre el cursor; el segundo une sus redes mientras está presionado. Ambos usan el solver nodal existente.
- Circuitos: `assets/Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json` (74138 + 7400, LED en Σm(1,2)) y `assets/Practica 7 - Daniel Gibran Alvarez Armienta - sumador.json` (74283 + 7448 + display de cátodo común + LED de C4).
- Ejemplos: `assets/ejemplo_74151.json`, `ejemplo_74157.json`, `ejemplo_7485.json`, `ejemplo_potenciometro.json`, `ejemplo_pulsador.json` y `ejemplo_analogico.json`.
- Generadores: `tools/gen_practica7.py` y `tools/gen_ejemplos_msi.py`; checklists de 8 y 256 casos.
- Verificación: `cargo test` da 52 pruebas correctas; el decodificador pasa 8/8 y el sumador 256/256. Los CLI `--simular`, `--bom`, `--instrucciones` y `--screenshot` funcionan con ambos circuitos.
- Capturas: `docs/screenshots/practica7_decoder.png` y `practica7_sumador.png`.

No se modificó el reporte de la Práctica 7 ni su guía de armado. La Práctica 6 sigue pasando 16/16 + 16/16.
