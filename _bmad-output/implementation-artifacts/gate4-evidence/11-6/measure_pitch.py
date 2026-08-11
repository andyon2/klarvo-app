#!/usr/bin/env python3
"""Misst den Zeilenabstand (line pitch) im Klarvo-Live-Preview-Panel eines Screenshots.

Verfahren (objektiv, kein Augenmass):
  1. ffmpeg schneidet den Transkript-Bereich aus und liefert ihn als 8-Bit-Graustufen-Rohbild.
  2. Pro Bildzeile wird gezaehlt, wie viele Pixel heller als THRESH sind (heller Text auf dunklem Panel).
  3. Zusammenhaengende Zeilenbaender oberhalb MIN_PIX bilden je eine Textzeile.
  4. Der Abstand aufeinanderfolgender Band-Schwerpunkte IST der gesuchte Zeilenabstand.

Aufruf: measure_pitch.py <screenshot.png> [x y w h]
"""
import subprocess
import sys

THRESH = 110      # Helligkeit, ab der ein Pixel als Textpixel gilt
MIN_PIX = 8       # Mindestzahl Textpixel, damit eine Bildzeile als "Text" zaehlt
MIN_BAND = 8      # Mindesthoehe eines Bandes in Pixeln (filtert Rauschen/Cursor)


def gray_rows(png, x, y, w, h):
    """Schneidet den Bereich aus und gibt ihn als Liste von Zeilen (je Liste von Grauwerten)."""
    cmd = ["ffmpeg", "-v", "error", "-i", png,
           "-vf", f"crop={w}:{h}:{x}:{y},format=gray",
           "-f", "rawvideo", "-pix_fmt", "gray", "-"]
    raw = subprocess.run(cmd, capture_output=True, check=True).stdout
    if len(raw) != w * h:
        sys.exit(f"Unerwartete Rohbildgroesse: {len(raw)} statt {w*h}")
    return [raw[r * w:(r + 1) * w] for r in range(h)]


def bands(rows):
    """Findet zusammenhaengende Textzeilen-Baender und gibt (start, ende, schwerpunkt) zurueck."""
    counts = [sum(1 for p in row if p > THRESH) for row in rows]
    out, start = [], None
    for i, c in enumerate(counts + [0]):
        if c >= MIN_PIX and start is None:
            start = i
        elif c < MIN_PIX and start is not None:
            if i - start >= MIN_BAND:
                # Schwerpunkt nach Textpixel-Masse, nicht geometrische Mitte:
                # robuster gegen Ober-/Unterlaengen einzelner Zeilen.
                mass = sum(counts[start:i])
                centroid = sum(r * counts[r] for r in range(start, i)) / mass
                out.append((start, i - 1, centroid))
            start = None
    return out


def main():
    png = sys.argv[1]
    x, y, w, h = (int(v) for v in (sys.argv[2:6] or [40, 1780, 980, 480]))
    bs = bands(gray_rows(png, x, y, w, h))
    print(f"Ausschnitt: x={x} y={y} w={w} h={h}  |  {len(bs)} Textzeilen erkannt")
    for i, (s, e, c) in enumerate(bs):
        print(f"  Zeile {i+1}: Zeilen {s}-{e} (Hoehe {e-s+1}px), Schwerpunkt y={c+y:.1f}")
    pitches = [bs[i + 1][2] - bs[i][2] for i in range(len(bs) - 1)]
    if pitches:
        avg = sum(pitches) / len(pitches)
        print("  Abstaende: " + ", ".join(f"{p:.1f}px" for p in pitches))
        print(f"  MITTLERER ZEILENABSTAND: {avg:.2f} px")
    else:
        print("  Zu wenige Zeilen fuer eine Abstandsmessung.")


if __name__ == "__main__":
    main()
