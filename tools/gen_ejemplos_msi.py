#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Ejemplos de los CI MSI y del modo analógico de OpenCircuit."""
from gen_practica7 import Circ, hole


def switch(c, sid, rid, col, net, board=2):
    c.add(sid, "interruptor", [hole(board, 5, col), hole(board, 5, col + 2)], cerrado=False)
    c.label("5V", board, 6, col)
    c.label(net, board, 6, col + 2)
    c.add(rid, "resistencia", [hole(board, 9, col + 2), f"b-{col+2}@{board}"], valor=10_000.0)


def rail(c, board=2):
    c.label_hole("5V", f"t+1@{board}")
    c.label_hole("GND", f"b-1@{board}")


def led_sink(c, n, col, net):
    """LED que enciende cuando la salida baja (sumidero)."""
    c.add(f"R{n}", "resistencia", [hole(1, 5, col), hole(1, 5, col + 1)], valor=330.0)
    c.label("5V", 1, 6, col)
    c.label(f"LED{n}_A", 1, 6, col + 1)
    c.add(f"D{n}", "led", [hole(1, 5, col + 2), hole(1, 5, col + 3)], color="rojo")
    c.label(f"LED{n}_A", 1, 6, col + 2)
    c.label(net, 1, 6, col + 3)


def led_source(c, n, col, net):
    """LED que enciende cuando la salida sube (fuente)."""
    c.add(f"R{n}", "resistencia", [hole(1, 5, col), hole(1, 5, col + 1)], valor=330.0)
    c.label(net, 1, 6, col)
    c.label(f"LED{n}_A", 1, 6, col + 1)
    c.add(f"D{n}", "led", [hole(1, 5, col + 2), hole(1, 5, col + 3)], color="verde")
    c.label(f"LED{n}_A", 1, 6, col + 2)
    c.label("GND", 1, 6, col + 3)


def base(nombre):
    c = Circ(nombre)
    c.add("F1", "fuente", ["t+1", "t-1"], valor=5.0)
    c.label_hole("5V", "t+2")
    c.label_hole("GND", "t-2")
    return c


def mux151():
    c = base("Ejemplo 74151 — multiplexor 8 a 1")
    u = c.chip("U1", "74151", 1, 1, 16)
    for pin, net in [(4, "D0"), (3, "D1"), (2, "D2"), (1, "D3"),
                     (15, "D4"), (14, "D5"), (13, "D6"), (12, "D7"),
                     (11, "A"), (10, "B"), (9, "C"), (7, "GND")]:
        c.near(u[pin], net)
    c.near(u[5], "Y")
    c.near(u[16], "5V")
    c.near(u[8], "GND")
    for i, net in enumerate("D0 D1 D2 D3 D4 D5 D6 D7 A B C".split()):
        switch(c, f"S{i+1}", f"RS{i+1}", 1 + 3 * i, net)
    rail(c)
    led_sink(c, 1, 40, "Y")
    c.save("ejemplo_74151.json")


def mux157():
    c = base("Ejemplo 74157 — cuatro multiplexores 2 a 1")
    u = c.chip("U1", "74157", 1, 1, 16)
    for pin, net in [(1, "S"), (15, "GND"), (2, "A1"), (3, "B1"),
                     (5, "A2"), (6, "B2"), (11, "A3"), (10, "B3"),
                     (14, "A4"), (13, "B4")]:
        c.near(u[pin], net)
    for pin, net in [(4, "Y1"), (7, "Y2"), (9, "Y3"), (12, "Y4")]:
        c.near(u[pin], net)
    c.near(u[16], "5V")
    c.near(u[8], "GND")
    for i, net in enumerate("A1 B1 A2 B2 A3 B3 A4 B4 S".split()):
        switch(c, f"S{i+1}", f"RS{i+1}", 1 + 3 * i, net)
    rail(c)
    for i, net in enumerate(["Y1", "Y2", "Y3", "Y4"]):
        led_sink(c, i + 1, 40 + 4 * i, net)
    c.save("ejemplo_74157.json")


def comparador():
    c = base("Ejemplo 7485 — comparador de magnitud de 4 bits")
    u = c.chip("U1", "7485", 1, 1, 16)
    for pin, net in [(10, "A0"), (12, "A1"), (13, "A2"), (15, "A3"),
                     (9, "B0"), (11, "B1"), (14, "B2"), (1, "B3"),
                     (2, "GND"), (3, "5V"), (4, "GND")]:
        c.near(u[pin], net)
    for pin, net in [(5, "GT"), (6, "EQ"), (7, "LT")]:
        c.near(u[pin], net)
    c.near(u[16], "5V")
    c.near(u[8], "GND")
    for i, net in enumerate("A0 A1 A2 A3 B0 B1 B2 B3".split()):
        switch(c, f"S{i+1}", f"RS{i+1}", 1 + 3 * i, net)
    rail(c)
    for i, net in enumerate(["GT", "EQ", "LT"]):
        led_source(c, i + 1, 40 + 4 * i, net)
    c.save("ejemplo_7485.json")


def potenciometro():
    c = base("Ejemplo — potenciómetro y LED")
    c.add("P1", "potenciometro", [hole(1, 5, 10), hole(1, 5, 12), hole(1, 5, 14)],
          valor=10_000.0, posicion=0.5)
    c.label("5V", 1, 6, 10)
    c.label("W", 1, 6, 12)
    c.label("GND", 1, 6, 14)
    c.add("R1", "resistencia", [hole(1, 5, 20), hole(1, 5, 21)], valor=330.0)
    c.label("W", 1, 6, 20)
    c.label("LED_A", 1, 6, 21)
    c.add("D1", "led", [hole(1, 5, 22), hole(1, 5, 23)], color="rojo")
    c.label("LED_A", 1, 6, 22)
    c.label("GND", 1, 6, 23)
    c.save("ejemplo_potenciometro.json")


def pulsador():
    c = base("Ejemplo — pulsador momentáneo")
    c.add("B1", "pulsador", [hole(1, 5, 10), hole(1, 5, 12)], presionado=False)
    c.label("5V", 1, 6, 10)
    c.label("P", 1, 6, 12)
    c.add("R1", "resistencia", [hole(1, 5, 14), hole(1, 5, 16)], valor=10_000.0)
    c.label("P", 1, 6, 14)
    c.label("GND", 1, 6, 16)
    c.add("R2", "resistencia", [hole(1, 5, 20), hole(1, 5, 21)], valor=330.0)
    c.label("P", 1, 6, 20)
    c.label("LED_A", 1, 6, 21)
    c.add("D1", "led", [hole(1, 5, 22), hole(1, 5, 23)], color="verde")
    c.label("LED_A", 1, 6, 22)
    c.label("GND", 1, 6, 23)
    c.save("ejemplo_pulsador.json")


def analogico():
    c = base("Ejemplo — divisor resistivo y LED")
    c.add("R1", "resistencia", [hole(1, 5, 10), hole(1, 5, 12)], valor=10_000.0)
    c.label("5V", 1, 6, 10)
    c.label("MID", 1, 6, 12)
    c.add("R2", "resistencia", [hole(1, 5, 14), hole(1, 5, 16)], valor=10_000.0)
    c.label("MID", 1, 6, 14)
    c.label("GND", 1, 6, 16)
    c.add("R3", "resistencia", [hole(1, 5, 20), hole(1, 5, 21)], valor=330.0)
    c.label("MID", 1, 6, 20)
    c.label("LED_A", 1, 6, 21)
    c.add("D1", "led", [hole(1, 5, 22), hole(1, 5, 23)], color="rojo")
    c.label("LED_A", 1, 6, 22)
    c.label("GND", 1, 6, 23)
    c.save("ejemplo_analogico.json")


if __name__ == "__main__":
    mux151()
    mux157()
    comparador()
    potenciometro()
    pulsador()
    analogico()
    print("6 ejemplos MSI/analógicos generados")
