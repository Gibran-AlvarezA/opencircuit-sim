# Alcance y roadmap de OpenCircuit

## Mínimo funcional

No existe una norma única para definir el mínimo de un simulador educativo. Para OpenCircuit se toma como mínimo una herramienta que permita:

1. Crear, abrir, guardar y recuperar un circuito sin perder cambios.
2. Ver una representación física de la protoboard y sus conexiones.
3. Elegir componentes básicos y conocer sus pines.
4. Conectar componentes mediante cables o redes etiquetadas.
5. Simular entradas y observar salidas.
6. Detectar errores eléctricos que impidan un armado seguro.
7. Generar instrucciones físicas reproducibles.
8. Obtener una lista de materiales y formatos exportables.
9. Deshacer, rehacer y recuperar un archivo ante cambios externos.
10. Ofrecer ayuda y un formato documentado para uso humano y por agentes.
11. Incluir pruebas reproducibles de los circuitos de ejemplo.

OpenCircuit cumple este mínimo para circuitos digitales de 5 V sobre protoboard. El potenciómetro y el pulsador están modelados dentro del solver nodal, pero la aplicación no pretende ser un simulador SPICE: no calcula transitorios, AC ni modelos analógicos continuos.

## Evidencia en el proyecto

| Capacidad | Implementación |
|---|---|
| Persistencia y recuperación | JSON compartido, autosave, snapshots y detección de cambios externos |
| Representación física | `src/draw.rs`, una a tres protoboards y componentes visibles |
| Componentes | `src/model.rs`, guía `--guia` y ejemplos en `assets/` |
| Conectividad | `src/route.rs`, cables, rieles y etiquetas de red |
| Simulación | `src/sim.rs`, estados lógicos, LEDs, display y tabla de verdad |
| Seguridad eléctrica | `src/erc.rs`, errores de alimentación, cortos, entradas y salidas |
| Armado físico | `src/instr.rs`, pestaña de instrucciones y exportación Markdown |
| Materiales | `src/bom.rs`, exportación CSV |
| Agentes | `src/mcp.rs`, servidor MCP por stdio y guía JSON |
| Regresión | Suite Rust con 54 pruebas y checklists de prácticas |

## Referencias usadas

Estas capacidades se contrastaron con:

- [CircuitVerse](https://circuitverse.org): construcción visual, zoom, copia/pega, análisis combinacional, testbench, subcircuitos y exportación.
- [Tinkercad Circuits](https://www.tinkercad.com/blog/official-guide-to-tinkercad-circuits): biblioteca de componentes, conexión y simulación interactiva.
- [Falstad Circuit Simulator](https://www.falstad.com/circuit): interacción en tiempo real, estado visible, edición de componentes y ejemplos.
- [KiCad](https://www.kicad.org): captura, bibliotecas, ERC, simulación y reportes; se usa como referencia de calidad EDA, no como alcance obligatorio.
- [Digital](https://github.com/hneemann/Digital): tablas de verdad, análisis de circuitos y documentación orientada a aprendizaje.

## Mejoras propuestas

### P1 — uso inmediato

1. Onboarding guiado con un circuito LED + resistencia.
2. Buscador, filtros y favoritos para componentes.
3. Inspector único para editar propiedades con validación.
4. Previsualización y arrastre para conectar cables.
5. Resaltado de redes y lectura de voltaje, corriente y estado lógico.
6. Panel de ERC con explicación y acción correctiva.
7. Mejoras de zoom, paneo, foco de teclado, escalado y accesibilidad.
8. Exportación de un paquete con JSON, Markdown, BOM, ERC y captura.

### P2 — aprendizaje y verificación

9. Tabla de verdad editable con comparación contra resultados esperados.
10. Diagrama temporal digital básico.
11. Modo lección con pasos, checkpoints y retroalimentación.
12. Subcircuitos reutilizables.
13. Instrucciones visuales con imágenes por paso y fotos reales autorizadas.

### P3 — robustez y distribución

14. MCP con dry-run, diff, transacciones y mensajes de validación para agentes.
15. Paquetes multiplataforma, diagnóstico de dependencias y exportación reproducible.

Cada mejora debe implementarse como un ticket separado, con prueba y criterio de aceptación. Las fotos del armado se añadirán únicamente cuando el usuario las entregue.
