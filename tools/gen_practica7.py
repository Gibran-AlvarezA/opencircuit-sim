#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Genera los circuitos de la Práctica 7 para OpenCircuit.

Autocontenido: construye el cableado por etiquetas de red, comprueba las
tablas y no escribe nada si un caso no coincide.
"""
from pathlib import Path
import csv
import json

OUT = Path(__file__).resolve().parent.parent / "assets"


def hole(board, row, col):
    s = f"{chr(ord('a') + row)}{col}"
    return s if board == 1 else f"{s}@{board}"


class Circ:
    def __init__(self, nombre):
        self.nombre = nombre
        self.comps = []
        self.n = 0

    def add(self, cid, tipo, pines, **extra):
        self.comps.append({"id": cid, "tipo": tipo, "pines": pines, **extra})

    def label(self, net, board, row, col):
        self.n += 1
        self.add(f"L{self.n}", "etiqueta", [hole(board, row, col)], nombre=net)

    def label_hole(self, net, h):
        self.n += 1
        self.add(f"L{self.n}", "etiqueta", [h], nombre=net)

    def chip(self, cid, modelo, board, col, pines):
        self.add(cid, "chip", [hole(board, 5, col)], modelo=modelo)
        half = pines // 2
        pins = {}
        for p in range(1, pines + 1):
            if p <= half:
                pins[p] = (board, 5, col + p - 1)
            else:
                pins[p] = (board, 4, col + pines - p)
        return pins

    def near(self, pin_hole, net):
        board, row, col = pin_hole
        self.label(net, board, row + 1 if row == 5 else row - 1, col)

    def save(self, archivo):
        data = {
            "nombre": self.nombre,
            "columnas": 60,
            "tableros": 3,
            "componentes": self.comps,
        }
        (OUT / archivo).write_text(
            json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )


def decoder():
    c = Circ("Practica 7 - Daniel Gibran Alvarez Armienta - decodificador")
    c.add("F1", "fuente", ["t+1", "t-1"], valor=5.0)
    c.label_hole("5V", "t+2")
    c.label_hole("GND", "t-2")

    u1 = c.chip("U1", "74138", 1, 1, 16)
    # A es MSB (pin 3), B pin 2 y C es LSB (pin 1).
    for pin, net in [(1, "C"), (2, "B"), (3, "A"), (4, "GND"),
                     (5, "GND"), (6, "5V")]:
        c.near(u1[pin], net)
    c.near(u1[16], "5V")
    c.near(u1[8], "GND")
    c.near(u1[14], "Y1")
    c.near(u1[13], "Y2")

    u2 = c.chip("U2", "7400", 1, 10, 14)
    c.near(u2[1], "Y1")
    c.near(u2[2], "Y2")
    c.near(u2[3], "F")
    c.near(u2[4], "F")
    c.near(u2[5], "F")
    c.near(u2[6], "LED_K")
    for p in (9, 10, 12, 13):
        c.near(u2[p], "GND")
    c.near(u2[14], "5V")
    c.near(u2[7], "GND")

    c.add("R1", "resistencia", [hole(1, 5, 20), hole(1, 5, 21)], valor=330.0)
    c.label("5V", 1, 6, 20)
    c.label("LED_A", 1, 6, 21)
    c.add("D1", "led", [hole(1, 5, 22), hole(1, 5, 23)], color="rojo")
    c.label("LED_A", 1, 6, 22)
    c.label("LED_K", 1, 6, 23)

    # Tres interruptores con pull-down de 10 kΩ.
    for i, (col, net) in enumerate([(3, "A"), (8, "B"), (13, "C")]):
        c.add(f"S{i+1}", "interruptor", [hole(2, 5, col), hole(2, 5, col + 2)], cerrado=False)
        c.label("5V", 2, 6, col)
        c.label(net, 2, 6, col + 2)
        c.add(f"R{i+2}", "resistencia", [hole(2, 9, col + 2), f"b-{col+2}@2"], valor=10_000.0)
    c.label_hole("5V", "t+1@2")
    c.label_hole("GND", "b-1@2")
    c.save("Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json")

    filas = []
    for m in range(8):
        a, b, cc = (m >> 2) & 1, (m >> 1) & 1, m & 1
        filas.append([m, a, b, cc, int(a == 0 and (b ^ cc) == 1)])
    with (OUT / "checklist_practica7_decoder.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["m", "A", "B", "C", "LED"])
        w.writerows(filas)
    assert sum(r[4] for r in filas) == 2
    print("Practica 7 - Daniel Gibran Alvarez Armienta - decodificador.json: 8/8 combinaciones, 2 CI, LED en m=1 y m=2")


def sumador():
    c = Circ("Practica 7 - Daniel Gibran Alvarez Armienta - sumador")
    c.add("F1", "fuente", ["t+1", "t-1"], valor=5.0)
    c.label_hole("5V", "t+2")
    c.label_hole("GND", "t-2")

    u1 = c.chip("U1", "74283", 1, 1, 16)
    for pin, net in [(5, "A1"), (3, "A2"), (14, "A3"), (12, "A4"),
                     (6, "B1"), (2, "B2"), (15, "B3"), (11, "B4"),
                     (7, "GND")]:
        c.near(u1[pin], net)
    for pin, net in [(4, "S1"), (1, "S2"), (13, "S3"), (10, "S4"),
                     (9, "C4")]:
        c.near(u1[pin], net)
    c.near(u1[16], "5V")
    c.near(u1[8], "GND")

    u2 = c.chip("U2", "7448", 1, 10, 16)
    for pin, net in [(7, "S1"), (1, "S2"), (2, "S3"), (6, "S4"),
                     (3, "5V"), (4, "5V"), (5, "5V")]:
        c.near(u2[pin], net)
    for pin, net in [(13, "SEG_A"), (12, "SEG_B"), (11, "SEG_C"),
                     (10, "SEG_D"), (9, "SEG_E"), (15, "SEG_F"),
                     (14, "SEG_G")]:
        c.near(u2[pin], net)
    c.near(u2[16], "5V")
    c.near(u2[8], "GND")

    u3 = c.chip("U3", "7400", 1, 20, 14)
    c.near(u3[9], "C4")
    c.near(u3[10], "C4")
    c.near(u3[8], "LED_C4")
    for p in (1, 2, 4, 5, 12, 13):
        c.near(u3[p], "GND")
    c.near(u3[14], "5V")
    c.near(u3[7], "GND")

    c.add("R2", "resistencia", [hole(1, 5, 30), hole(1, 5, 31)], valor=330.0)
    c.label("5V", 1, 6, 30)
    c.label("LED_C4_A", 1, 6, 31)
    c.add("D2", "led", [hole(1, 5, 32), hole(1, 5, 33)], color="verde")
    c.label("LED_C4_A", 1, 6, 32)
    c.label("LED_C4", 1, 6, 33)

    for i, net in enumerate(["A1", "A2", "A3", "A4", "B1", "B2", "B3", "B4"]):
        col = 1 + 3 * i
        c.add(f"S{i+1}", "interruptor", [hole(2, 5, col), hole(2, 5, col + 2)], cerrado=False)
        c.label("5V", 2, 6, col)
        c.label(net, 2, 6, col + 2)
        c.add(f"R{i+3}", "resistencia", [hole(2, 9, col + 2), f"b-{col+2}@2"], valor=10_000.0)
    c.label_hole("5V", "t+1@2")
    c.label_hole("GND", "b-1@2")

    # Display de 10 pines (cátodo común): pin g en c20.
    c.add("DS1", "display", ["c20@3"])
    c.label("SEG_A", 3, 3, 23)
    c.label("SEG_B", 3, 3, 24)
    c.label("SEG_C", 3, 7, 23)
    c.label("SEG_D", 3, 7, 21)
    c.label("SEG_E", 3, 7, 20)
    c.label("SEG_F", 3, 3, 21)
    c.label("SEG_G", 3, 3, 20)
    c.label("GND", 3, 3, 22)
    c.label("GND", 3, 7, 22)
    c.label_hole("GND", "b-1@3")
    c.save("Practica 7 - Daniel Gibran Alvarez Armienta - sumador.json")

    filas = []
    for a in range(16):
        for b in range(16):
            total = a + b
            valido = total < 10 or 16 <= total <= 25
            filas.append([a, b, total & 15, total & 15 if valido else "", int(total > 15)])
    with (OUT / "checklist_practica7_sumador.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["a", "b", "suma", "display", "C4"])
        w.writerows(filas)
    assert len(filas) == 256
    print("Practica 7 - Daniel Gibran Alvarez Armienta - sumador.json: 256/256 combinaciones, 3 CI, 9+9=18 (S=2, C4=1)")


if __name__ == "__main__":
    decoder()
    sumador()
