#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Genera los circuitos de la Práctica 6 (OpenCircuit).

Autocontenido: los netlists van copiados aquí, ya verificados en Digital.
Si algo no cuadra con la tabla esperada, falla y no escribe nada.
"""
import itertools
import json
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "assets"

MODEL = {"7408": "74HC08", "7432": "74HC32", "7404": "74HC04"}
PINS = {
    "7408": {1: (1, 2, 3), 2: (4, 5, 6), 3: (9, 10, 8), 4: (12, 13, 11)},
    "7432": {1: (1, 2, 3), 2: (4, 5, 6), 3: (9, 10, 8), 4: (12, 13, 11)},
    "7404": {1: (1, 2), 2: (3, 4), 3: (5, 6), 4: (9, 8), 5: (11, 10), 6: (13, 12)},
}

RUMBO = {
    "archivo": "Practica 6 - Daniel Gibran Alvarez Armienta - rumbo.json",
    "nombre": "Practica 6 - Daniel Gibran Alvarez Armienta - rumbo",
    "logic": [
        ("na", "NOT", ("A",)), ("nb", "NOT", ("B",)),
        ("nc", "NOT", ("C",)), ("nd", "NOT", ("D",)),
        ("cd", "AND", ("C", "D")), ("ab", "AND", ("A", "B")),
        ("ad", "AND", ("A", "D")), ("bd", "AND", ("B", "D")),
        ("aB", "AND", ("na", "nb")), ("aBC", "AND", ("aB", "nc")),
        ("bc", "AND", ("nb", "nc")), ("bcd", "AND", ("bc", "nd")),
        ("bC", "AND", ("nb", "C")), ("seg_b", "AND", ("bC", "nd")),
        ("aC", "AND", ("na", "C")), ("seg_c", "AND", ("aC", "nd")),
        ("aCc", "AND", ("na", "nc")), ("aCcD", "AND", ("aCc", "nd")),
        ("aBCD", "AND", ("aBC", "nd")),
        ("oa1", "OR", ("A", "cd")), ("seg_a", "OR", ("oa1", "bcd")),
        ("od1", "OR", ("B", "aCcD")), ("seg_d", "OR", ("od1", "cd")),
        ("oe1", "OR", ("aBC", "bd")), ("oe2", "OR", ("oe1", "cd")),
        ("seg_e", "OR", ("oe2", "ab")),
        ("of1", "OR", ("aBC", "ad")), ("of2", "OR", ("of1", "cd")),
        ("seg_f", "OR", ("of2", "ab")),
        ("og1", "OR", ("ab", "cd")), ("seg_g", "OR", ("og1", "aBCD")),
    ],
    "place": [
        ("U1", "7408", [("cd", 1), ("ab", 2), ("ad", 3), ("bd", 4)]),
        ("U2", "7408", [("aB", 1), ("aBC", 2), ("bc", 3), ("bcd", 4)]),
        ("U3", "7408", [("bC", 1), ("seg_b", 2), ("aC", 3), ("seg_c", 4)]),
        ("U4", "7408", [("aCc", 1), ("aCcD", 2), ("aBCD", 3)]),
        ("U5", "7432", [("oa1", 1), ("seg_a", 2), ("od1", 3), ("seg_d", 4)]),
        ("U6", "7432", [("oe1", 1), ("oe2", 2), ("seg_e", 3), ("of1", 4)]),
        ("U7", "7432", [("of2", 1), ("seg_f", 2), ("og1", 3), ("seg_g", 4)]),
        ("U8", "7404", [("na", 1), ("nb", 2), ("nc", 3), ("nd", 4)]),
    ],
    # fila 0 = tablero 1
    "layout": [
        ("U1", 0, 1), ("U2", 0, 8), ("U3", 0, 15), ("U4", 0, 22), ("U8", 0, 29),
        ("U5", 1, 1), ("U6", 1, 8), ("U7", 1, 15),
    ],
    "expected": {
        0: (1, 0, 0, 1, 1, 1, 1), 1: (0, 0, 0, 0, 1, 1, 0),
        2: (0, 1, 1, 0, 0, 0, 0), 3: (1, 0, 0, 1, 1, 1, 1),
        4: (0, 0, 0, 1, 0, 0, 0), 5: (0, 0, 0, 1, 1, 0, 0),
        6: (0, 0, 1, 1, 0, 0, 0), 7: (1, 0, 0, 1, 1, 1, 1),
        8: (1, 0, 0, 0, 0, 0, 0), 9: (1, 0, 0, 0, 0, 1, 0),
        10: (1, 1, 0, 0, 0, 0, 0), 11: (1, 0, 0, 1, 1, 1, 1),
        12: (1, 0, 0, 1, 1, 1, 1), 13: (1, 0, 0, 1, 1, 1, 1),
        14: (1, 0, 0, 1, 1, 1, 1), 15: (1, 0, 0, 1, 1, 1, 1),
    },
}

CELULAR = {
    "archivo": "Practica 6 - Daniel Gibran Alvarez Armienta - celular.json",
    "nombre": "Practica 6 - Daniel Gibran Alvarez Armienta - celular",
    "logic": [
        ("nb", "NOT", ("B",)), ("nc", "NOT", ("C",)), ("nd", "NOT", ("D",)),
        ("t1", "AND", ("B", "D")), ("t2", "AND", ("nb", "nd")),
        ("t3", "AND", ("C", "D")), ("t4", "AND", ("nc", "nd")),
        ("t5", "AND", ("C", "nd")), ("t6", "AND", ("nb", "C")),
        ("t7", "AND", ("B", "nc")), ("t8", "AND", ("t7", "D")),
        ("t9", "AND", ("B", "nd")),
        ("oa1", "OR", ("A", "C")), ("oa2", "OR", ("oa1", "t2")),
        ("seg_a", "OR", ("oa2", "t1")),
        ("ob1", "OR", ("t3", "nb")), ("seg_b", "OR", ("ob1", "t4")),
        ("oc1", "OR", ("B", "nc")), ("seg_c", "OR", ("oc1", "D")),
        ("od1", "OR", ("t5", "t6")), ("od2", "OR", ("od1", "A")),
        ("od3", "OR", ("od2", "t8")), ("seg_d", "OR", ("od3", "t2")),
        ("seg_e", "OR", ("t5", "t2")),
        ("of1", "OR", ("t4", "A")), ("of2", "OR", ("of1", "t9")),
        ("seg_f", "OR", ("of2", "t7")),
        ("og1", "OR", ("t5", "t6")), ("og2", "OR", ("og1", "A")),
        ("seg_g", "OR", ("og2", "t7")),
    ],
    "place": [
        ("U1", "7408", [("t1", 1), ("t2", 2), ("t3", 3), ("t4", 4)]),
        ("U2", "7408", [("t5", 1), ("t6", 2), ("t7", 3), ("t8", 4)]),
        ("U3", "7408", [("t9", 1)]),
        ("U4", "7432", [("oa1", 1), ("oa2", 2), ("seg_a", 3), ("ob1", 4)]),
        ("U5", "7432", [("seg_b", 1), ("oc1", 2), ("seg_c", 3), ("od1", 4)]),
        ("U6", "7432", [("od2", 1), ("od3", 2), ("seg_d", 3), ("seg_e", 4)]),
        ("U7", "7432", [("of1", 1), ("of2", 2), ("seg_f", 3), ("og1", 4)]),
        ("U8", "7432", [("og2", 1), ("seg_g", 2)]),
        ("U9", "7404", [("nb", 1), ("nc", 2), ("nd", 3)]),
    ],
    "layout": [
        ("U1", 0, 1), ("U2", 0, 8), ("U3", 0, 15), ("U9", 0, 22),
        ("U4", 1, 1), ("U5", 1, 8), ("U6", 1, 15), ("U7", 1, 22), ("U8", 1, 29),
    ],
    "patterns": {
        0: (1, 1, 1, 1, 1, 1, 0), 1: (0, 1, 1, 0, 0, 0, 0),
        2: (1, 1, 0, 1, 1, 0, 1), 3: (1, 1, 1, 1, 0, 0, 1),
        4: (0, 1, 1, 0, 0, 1, 1), 5: (1, 0, 1, 1, 0, 1, 1),
        6: (1, 0, 1, 1, 1, 1, 1), 7: (1, 1, 1, 0, 0, 0, 0),
        8: (1, 1, 1, 1, 1, 1, 1), 9: (1, 1, 1, 1, 0, 1, 1),
    },
}


def gates(spec):
    """Expande PLACEMENT a compuertas con sus pines reales."""
    by_id = {gid: (kind, ins) for gid, kind, ins in spec["logic"]}
    out, used = [], set()
    for chip, model, units in spec["place"]:
        for gid, unit in units:
            if gid in used:
                raise SystemExit(f"{gid} colocado dos veces")
            kind, ins = by_id[gid]
            pins = PINS[model][unit]
            if len(ins) != len(pins) - 1:
                raise SystemExit(f"{gid}: entradas y pines no coinciden")
            used.add(gid)
            out.append({"id": gid, "kind": kind, "inputs": ins, "output": gid,
                        "chip": chip, "unit": unit, "pin_in": pins[:-1],
                        "pin_out": pins[-1]})
    missing = set(by_id) - used
    if missing:
        raise SystemExit(f"compuertas sin colocar: {sorted(missing)}")
    return out


def evaluate(spec, bits):
    """Recorre LOGIC en orden (ya es topológico), no PLACEMENT."""
    val = dict(zip("ABCD", bits))
    for gid, kind, ins in spec["logic"]:
        vals = [val[i] for i in ins]
        if kind == "AND":
            val[gid] = int(all(vals))
        elif kind == "OR":
            val[gid] = int(any(vals))
        else:
            val[gid] = int(not vals[0])
    return val


def truth(spec):
    rows = []
    for bits in itertools.product((0, 1), repeat=4):
        v = evaluate(spec, bits)
        rows.append({"m": bits[0] * 8 + bits[1] * 4 + bits[2] * 2 + bits[3],
                     "segs": tuple(v["seg_" + s] for s in "abcdefg")})
    return rows


def net_label(name):
    """Las etiquetas se comparan sin mayúsculas: 'ab' y 'aB' chocarían."""
    if name in ("A", "B", "C", "D") or name.startswith("seg_"):
        return name.upper()
    if len(name) == 2 and name[0] == "n" and name[1] in "abcd":
        return name[1].upper() + "'"
    return "".join(ch.upper() + "'" if ch in "abcd" else ch.upper() for ch in name)


def hole(board, row, col):
    n = f"{chr(ord('a') + row)}{col}"
    return n if board == 1 else f"{n}@{board}"


def build(spec):
    gs = gates(spec)
    rows = truth(spec)
    expected = spec.get("expected") or spec["patterns"]
    for r in rows:
        if r["m"] in expected and r["segs"] != expected[r["m"]]:
            raise SystemExit(f"{spec['archivo']}: m{r['m']} -> {r['segs']}, "
                             f"esperado {expected[r['m']]}")

    cols, tableros = 60, 3
    pin_hole = {}
    for chip, board, col in spec["layout"]:
        for pin in range(1, 15):
            row = 5 if pin <= 7 else 4
            pin_hole[(chip, pin)] = (board + 1, row,
                                     col + pin - 1 if pin <= 7 else col + 14 - pin)

    comps, lid = [], 0

    def add(cid, tipo, pines, **extra):
        comps.append({"id": cid, "tipo": tipo, "pines": pines, **extra})

    def label(board, row, col, net):
        nonlocal lid
        lid += 1
        add(f"L{lid}", "etiqueta", [hole(board, row, col)], nombre=net_label(net))

    def label_at_pin(chip, pin, net):
        board, row, col = pin_hole[(chip, pin)]
        label(board, 6 if row == 5 else 3, col, net)

    add("F1", "fuente", ["t+1", "t-1"], valor=5.0)
    for chip, board, col in spec["layout"]:
        model = next(m for c, m, _ in spec["place"] if c == chip)
        add(chip, "chip", [hole(board + 1, 5, col)], modelo=MODEL[model])

    # tablero 3: interruptores A-D, display y resistencias de segmento
    for i, col in enumerate([1, 4, 7, 10]):
        add(f"S{i + 1}", "interruptor", [hole(3, 7, col), hole(3, 7, col + 1)])
        add(f"R{i + 1}", "resistencia", [hole(3, 8, col + 1), f"b-{col + 1}@3"],
            valor=10000.0)
        add(f"W{i + 1}", "cable", [hole(3, 9, col), f"t+{col + 2}@3"], color="rojo")
        label(3, 9, col + 1, "ABCD"[i])

    for i in range(7):
        c = 20 + 2 * i
        add(f"R{5 + i}", "resistencia", [hole(3, 8, c), hole(3, 8, c + 1)],
            valor=470.0)
        label(3, 9, c + 1, "seg_" + "abcdefg"[i])
    add("DS1", "display", [hole(3, 7, 20 + 2 * i) for i in range(7)] + [hole(3, 7, 34)])
    add("W9", "cable", [hole(3, 8, 34), "b-34@3"], color="negro")

    # alimentación: rieles por tablero y unión entre tableros
    for b in (1, 2, 3):
        add(f"Wx{b}a", "cable", [f"t+60@{b}", f"b+60@{b}"], color="rojo")
        add(f"Wx{b}b", "cable", [f"t-60@{b}", f"b-60@{b}"], color="negro")
    for b in (2, 3):
        add(f"Wy{b}a", "cable", [f"t+{56 + b}@1", f"t+1@{b}"], color="rojo")
        add(f"Wy{b}b", "cable", [f"t-{56 + b}@1", f"t-1@{b}"], color="negro")
    for chip, board, col in spec["layout"]:
        add(f"Wv{chip}", "cable", [hole(board + 1, 3, col), f"t+{col + 8}@{board + 1}"],
            color="rojo")
        add(f"Wg{chip}", "cable", [hole(board + 1, 6, col + 6), f"b-{col + 8}@{board + 1}"],
            color="negro")

    for g in gs:
        for pin, net in zip(g["pin_in"], g["inputs"]):
            label_at_pin(g["chip"], pin, net)
        label_at_pin(g["chip"], g["pin_out"], g["output"])

    # compuertas sobrantes: entradas a GND para que no queden al aire
    for chip, model, units in spec["place"]:
        used = {u for _, u in units}
        for unit, pins in PINS[model].items():
            if unit not in used:
                for p in pins[:-1]:
                    label_at_pin(chip, p, "GND")
    add(f"L{lid + 1}", "etiqueta", ["b-1"], nombre="GND")

    circ = {"nombre": spec["nombre"], "columnas": cols, "tableros": tableros,
            "componentes": comps}
    path = OUT / spec["archivo"]
    path.write_text(json.dumps(circ, ensure_ascii=False, indent=2) + "\n")

    ok = sum(1 for r in rows if r["m"] in expected)
    chips = len(spec["layout"])
    casos = f"{ok}/{len(expected)} casos"
    if "patterns" in spec:
        casos += f" y {len(rows) - ok} no importa"
    print(f"{spec['archivo']}: {casos}, {chips} CI, {len(comps)} componentes")


if __name__ == "__main__":
    build(RUMBO)
    build(CELULAR)
