#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Genera el proyecto final de Circuitos Digitales para OpenCircuit.

Sumador-restador de 4 bits con entradas y salidas BCD. El asset usa solo
etiquetas de red; la comprobación de los 512 casos válidos se hace aquí y en
src/sim.rs.
"""
from pathlib import Path
import csv
import json

OUT = Path(__file__).resolve().parent.parent / "assets"
NOMBRE = "Proyecto Final - Daniel Gibran Alvarez Armienta y Jose Salvador Arreguin Verdugo"


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


def bcd_parts(n):
    return divmod(n, 10)


def switch_bits(n):
    d, u = bcd_parts(n)
    return [d, (u >> 3) & 1, (u >> 2) & 1, (u >> 1) & 1, u & 1]


def bcd_a_binario(bits):
    return bits[4] + 2 * bits[3] + 4 * bits[2] + 8 * bits[1] + 10 * bits[0]


def sumador_restador(x, y, s):
    yx = y ^ (0b1111 if s else 0)
    low = (x & 0b1111) + (yx & 0b1111) + s
    c4 = (low >> 4) & 1
    return ((s ^ c4) << 4) | (low & 0b1111), c4


def magnitud(r, signo):
    return (r ^ (0b11111 if signo else 0)) + signo


def correccion(r):
    if r < 5:
        return 0
    if r < 10:
        return 3
    if r < 15:
        return 6
    return 9


def bin_a_bcd(m):
    u0 = m & 1
    r = m >> 1
    t = r + correccion(r)
    dec = (t >> 3) & 3
    uni = ((t & 7) << 1) | u0
    return dec, uni


def evaluar(a, b, s):
    x = bcd_a_binario(switch_bits(a))
    y = bcd_a_binario(switch_bits(b))
    r, _ = sumador_restador(x, y, s)
    signo = 1 if s and (r >> 4) & 1 else 0
    m = magnitud(r, signo)
    dec, uni = bin_a_bcd(m)
    return dec, uni, signo


def proyecto_final():
    c = Circ(NOMBRE)
    c.add("F1", "fuente", ["t+1", "t-1"], valor=5.0)
    c.label_hole("5V", "t+2")
    c.label_hole("GND", "t-2")
    c.label_hole("GND", "b-1")

    # Tablero 1: entradas y conversión BCD->binario. Los interruptores van en
    # columnas libres (1-33) para no compartir columna con los chips (34-57).
    u1 = c.chip("U1", "74283", 1, 34, 16)
    u2 = c.chip("U2", "74283", 1, 42, 16)
    u3 = c.chip("U3", "74283", 1, 50, 16)
    u6 = c.chip("U6", "7486", 3, 1, 14)
    u7 = c.chip("U7", "7486", 3, 8, 14)
    u8 = c.chip("U8", "7486", 3, 15, 14)
    u9 = c.chip("U9", "7408", 3, 22, 14)

    # U1: X = AU + 10*AD.  A1..A4 son U0..U3; B2=B4=AD.
    for pin, net in [(5, "AU0"), (3, "AU1"), (14, "AU2"), (12, "AU3"),
                     (6, "GND"), (2, "AD"), (15, "GND"), (11, "AD"),
                     (7, "GND"), (4, "BX0"), (1, "BX1"), (13, "BX2"),
                     (10, "BX3")]:
        c.near(u1[pin], net)
    c.near(u1[16], "5V")
    c.near(u1[8], "GND")

    # U2: Y = BU + 10*BD.
    for pin, net in [(5, "BU0"), (3, "BU1"), (14, "BU2"), (12, "BU3"),
                     (6, "GND"), (2, "BD"), (15, "GND"), (11, "BD"),
                     (7, "GND"), (4, "BY0"), (1, "BY1"), (13, "BY2"),
                     (10, "BY3")]:
        c.near(u2[pin], net)
    c.near(u2[16], "5V")
    c.near(u2[8], "GND")

    # U3: suma/resta de 4 bits. El quinto bit R4 = S xor C4 se arma en U8.
    for pin, net in [(5, "BX0"), (3, "BX1"), (14, "BX2"), (12, "BX3"),
                     (6, "Y0S"), (2, "Y1S"), (15, "Y2S"), (11, "Y3S"),
                     (7, "S"), (4, "R0"), (1, "R1"), (13, "R2"),
                     (10, "R3"), (9, "C4")]:
        c.near(u3[pin], net)
    c.near(u3[16], "5V")
    c.near(u3[8], "GND")

    # U6: Y' = Y xor S.
    for pin, net in [(1, "BY0"), (2, "S"), (3, "Y0S"),
                     (4, "BY1"), (5, "S"), (6, "Y1S"),
                     (9, "BY2"), (10, "S"), (8, "Y2S"),
                     (12, "BY3"), (13, "S"), (11, "Y3S")]:
        c.near(u6[pin], net)
    c.near(u6[14], "5V")
    c.near(u6[7], "GND")

    # U7: R xor signo (bits 0..3).
    for pin, net in [(1, "R0"), (2, "SIG"), (3, "XR0"),
                     (4, "R1"), (5, "SIG"), (6, "XR1"),
                     (9, "R2"), (10, "SIG"), (8, "XR2"),
                     (12, "R3"), (13, "SIG"), (11, "XR3")]:
        c.near(u7[pin], net)
    c.near(u7[14], "5V")
    c.near(u7[7], "GND")

    # U8: R4 = S xor C4; XR4 = R4 xor signo; M4 = XR4 xor MC4.
    for pin, net in [(1, "S"), (2, "C4"), (3, "R4"),
                     (4, "R4"), (5, "SIG"), (6, "XR4"),
                     (9, "XR4"), (10, "MC4"), (8, "M4"),
                     (12, "GND"), (13, "GND")]:
        c.near(u8[pin], net)
    c.near(u8[14], "5V")
    c.near(u8[7], "GND")

    # U9: signo = S*R4; N3 = M4*M3*M2*M1.
    for pin, net in [(1, "S"), (2, "R4"), (3, "SIG"),
                     (4, "M4"), (5, "M3"), (6, "P32"),
                     (9, "M2"), (10, "M1"), (8, "P10"),
                     (12, "P32"), (13, "P10"), (11, "N3")]:
        c.near(u9[pin], net)
    c.near(u9[14], "5V")
    c.near(u9[7], "GND")

    # Tablero 2: magnitud, corrección, compuertas y decodificadores.
    u4 = c.chip("U4", "74283", 2, 1, 16)
    u5 = c.chip("U5", "74283", 2, 9, 16)
    u10 = c.chip("U10", "7408", 2, 33, 14)
    u11 = c.chip("U11", "7408", 2, 40, 14)
    u12 = c.chip("U12", "7408", 2, 47, 14)
    u13 = c.chip("U13", "7432", 2, 54, 14)
    u14 = c.chip("U14", "7432", 3, 29, 14)

    # U4: magnitud, suma de XR + signo.
    for pin, net in [(5, "XR0"), (3, "XR1"), (14, "XR2"), (12, "XR3"),
                     (6, "GND"), (2, "GND"), (15, "GND"), (11, "GND"),
                     (7, "SIG"), (4, "M0"), (1, "M1"), (13, "M2"),
                     (10, "M3"), (9, "MC4")]:
        c.near(u4[pin], net)
    c.near(u4[16], "5V")
    c.near(u4[8], "GND")

    # U5: corrección R + c. c = N3 N2 N1 N0.
    for pin, net in [(5, "M1"), (3, "M2"), (14, "M3"), (12, "M4"),
                     (6, "N0"), (2, "N1"), (15, "N2"), (11, "N3"),
                     (7, "GND"), (4, "T0"), (1, "T1"), (13, "T2"),
                     (10, "T3"), (9, "T4")]:
        c.near(u5[pin], net)
    c.near(u5[16], "5V")
    c.near(u5[8], "GND")

    # U10: G10 = M4*(M3+M2); N2 = G10*~N3; P2 = M3*(M1+M2); N1 = G5*~N3.
    for pin, net in [(1, "M4"), (2, "O21"), (3, "G10"),
                     (4, "G10"), (5, "NOTN3"), (6, "N2"),
                     (9, "M3"), (10, "O01"), (8, "P2"),
                     (12, "G5"), (13, "NOTN3"), (11, "N1")]:
        c.near(u10[pin], net)
    c.near(u10[14], "5V")
    c.near(u10[7], "GND")

    # U11: N0 = P2*~M4 + M4*~M2*~M3 + M1*M2*M3 (los dos últimos en U12).
    for pin, net in [(1, "P2"), (2, "NOTM4"), (3, "K1"),
                     (4, "NOTM2"), (5, "NOTM3"), (6, "Q"),
                     (9, "M4"), (10, "Q"), (8, "K2"),
                     (12, "GND"), (13, "GND")]:
        c.near(u11[pin], net)
    c.near(u11[14], "5V")
    c.near(u11[7], "GND")

    # U12: P01 = M1*M2; K3 = P01*M3.
    for pin, net in [(1, "M1"), (2, "M2"), (3, "P01"),
                     (4, "P01"), (5, "M3"), (6, "K3"),
                     (9, "GND"), (10, "GND"),
                     (12, "GND"), (13, "GND")]:
        c.near(u12[pin], net)
    c.near(u12[14], "5V")
    c.near(u12[7], "GND")

    # U13: O21 = M3+M2; O01 = M1+M2; G5 = M4+P2.
    for pin, net in [(1, "M3"), (2, "M2"), (3, "O21"),
                     (4, "M1"), (5, "M2"), (6, "O01"),
                     (9, "M4"), (10, "P2"), (8, "G5"),
                     (12, "GND"), (13, "GND")]:
        c.near(u13[pin], net)
    c.near(u13[14], "5V")
    c.near(u13[7], "GND")

    # U14: N0 = K1 + K2 + K3.
    for pin, net in [(1, "K1"), (2, "K2"), (3, "O12"),
                     (4, "O12"), (5, "K3"), (6, "N0"),
                     (9, "GND"), (10, "GND"),
                     (12, "GND"), (13, "GND")]:
        c.near(u14[pin], net)
    c.near(u14[14], "5V")
    c.near(u14[7], "GND")

    # LED de signo: 5 V -> 330 ohm -> LED -> salida del inversor (sinking).
    # Va en las columnas libres 57-60 del tablero 3.
    c.add("R12", "resistencia", [hole(3, 9, 57), hole(3, 9, 58)], valor=330.0)
    c.label("5V", 3, 8, 57)
    c.label("LED_A", 3, 8, 58)
    c.add("D1", "led", [hole(3, 9, 59), hole(3, 9, 60)], color="rojo")
    c.label("LED_A", 3, 8, 59)
    c.label("LED_K", 3, 8, 60)
    c.label_hole("5V", "t+1@2")
    c.label_hole("GND", "b-1@2")

    # Tablero 3: inversores, decodificadores y displays.
    u15 = c.chip("U15", "7404", 3, 36, 14)
    u16 = c.chip("U16", "7448", 2, 17, 16)
    u17 = c.chip("U17", "7448", 2, 25, 16)

    for pin, net in [(1, "N3"), (2, "NOTN3"),
                     (3, "M4"), (4, "NOTM4"),
                     (5, "M2"), (6, "NOTM2"),
                     (9, "M3"), (8, "NOTM3"),
                     (11, "SIG"), (10, "LED_K"),
                     (13, "GND")]:
        c.near(u15[pin], net)
    c.near(u15[14], "5V")
    c.near(u15[7], "GND")

    # U16: decenas. T3 y T4 son los dos bits de la decena.
    for pin, net in [(7, "T3"), (1, "T4"), (2, "GND"), (6, "GND"),
                     (3, "5V"), (4, "5V"), (5, "5V"),
                     (13, "TENS_A"), (12, "TENS_B"), (11, "TENS_C"),
                     (10, "TENS_D"), (9, "TENS_E"), (15, "TENS_F"),
                     (14, "TENS_G")]:
        c.near(u16[pin], net)
    c.near(u16[16], "5V")
    c.near(u16[8], "GND")

    # U17: unidades. {T2,T1,T0,M0}.
    for pin, net in [(7, "M0"), (1, "T0"), (2, "T1"), (6, "T2"),
                     (3, "5V"), (4, "5V"), (5, "5V"),
                     (13, "UNI_A"), (12, "UNI_B"), (11, "UNI_C"),
                     (10, "UNI_D"), (9, "UNI_E"), (15, "UNI_F"),
                     (14, "UNI_G")]:
        c.near(u17[pin], net)
    c.near(u17[16], "5V")
    c.near(u17[8], "GND")

    c.add("DS1", "display", ["c45@3"])
    for net, row, col in [("TENS_A", 3, 48), ("TENS_B", 3, 49),
                          ("TENS_C", 7, 48), ("TENS_D", 7, 46),
                          ("TENS_E", 7, 45), ("TENS_F", 3, 46),
                          ("TENS_G", 3, 45), ("GND", 3, 47), ("GND", 7, 47)]:
        c.label(net, 3, row, col)

    c.add("DS2", "display", ["c52@3"])
    for net, row, col in [("UNI_A", 3, 55), ("UNI_B", 3, 56),
                          ("UNI_C", 7, 55), ("UNI_D", 7, 53),
                          ("UNI_E", 7, 52), ("UNI_F", 3, 53),
                          ("UNI_G", 3, 52), ("GND", 3, 54), ("GND", 7, 54)]:
        c.label(net, 3, row, col)

    c.label_hole("5V", "t+1@3")
    c.label_hole("GND", "b-1@3")

    # Los 11 interruptores: A (5), B (5) y S (1).
    entradas = ["AD", "AU3", "AU2", "AU1", "AU0",
                "BD", "BU3", "BU2", "BU1", "BU0", "S"]
    for i, net in enumerate(entradas):
        col = 1 + 3 * i
        c.add(f"S{i+1}", "interruptor", [hole(1, 9, col), hole(1, 9, col + 2)],
              cerrado=False)
        c.label("5V", 1, 8, col)
        c.label(net, 1, 8, col + 2)
        c.add(f"R{i+1}", "resistencia", [hole(1, 7, col + 2), f"b-{col+2}@1"],
              valor=10_000.0)

    c.save(NOMBRE + ".json")

    filas = []
    for a in range(16):
        for b in range(16):
            for s in (0, 1):
                dec, uni, signo = evaluar(a, b, s)
                filas.append([a, b, s, *switch_bits(a), *switch_bits(b), s,
                              dec, uni, signo])
    with (OUT / "checklist_proyecto_final.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["a", "b", "s", "ad", "au3", "au2", "au1", "au0",
                    "bd", "bu3", "bu2", "bu1", "bu0", "sw_s",
                    "dec", "uni", "signo"])
        w.writerows(filas)
    assert len(filas) == 512
    for a in range(16):
        for b in range(16):
            for s in (0, 1):
                esperado = a + b if s == 0 else a - b
                assert evaluar(a, b, s) == (
                    abs(esperado) // 10, abs(esperado) % 10, int(esperado < 0)
                ), (a, b, s, evaluar(a, b, s), esperado)
    print(f"{NOMBRE}.json: 512/512 casos válidos, 17 CI, 11 interruptores, "
          "2 displays y LED de signo")


if __name__ == "__main__":
    proyecto_final()
