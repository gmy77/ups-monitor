#!/usr/bin/env python3
"""Genera assets/ups-monitor.ico: fulmine bianco su quadrato arrotondato blu-viola."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

LATO = 256


def disegna(lato=LATO):
    img = Image.new("RGBA", (lato, lato), (0, 0, 0, 0))
    # Sfondo con gradiente diagonale #2f80ed -> #7b5cff
    sfondo = Image.new("RGBA", (lato, lato))
    px = sfondo.load()
    a, b = (0x2F, 0x80, 0xED), (0x7B, 0x5C, 0xFF)
    for y in range(lato):
        for x in range(lato):
            t = (x + y) / (2 * (lato - 1))
            px[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3)) + (255,)
    maschera = Image.new("L", (lato, lato), 0)
    ImageDraw.Draw(maschera).rounded_rectangle([8, 8, lato - 9, lato - 9], radius=lato // 5, fill=255)
    img.paste(sfondo, (0, 0), maschera)

    # Fulmine con leggero alone
    s = lato / 256
    punti = [(146, 28), (66, 142), (118, 142), (102, 228), (190, 106), (136, 106), (158, 28)]
    punti = [(x * s, y * s) for x, y in punti]
    alone = Image.new("RGBA", (lato, lato), (0, 0, 0, 0))
    ImageDraw.Draw(alone).polygon(punti, fill=(255, 255, 255, 140))
    img.alpha_composite(alone.filter(ImageFilter.GaussianBlur(6 * s)))
    ImageDraw.Draw(img).polygon(punti, fill=(255, 255, 255, 255))
    return img


if __name__ == "__main__":
    out = Path(__file__).with_name("ups-monitor.ico")
    disegna().save(out, sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print(out)
