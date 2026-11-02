import math
import os
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("assets/shards", exist_ok=True)

# -------------------------------------------------------------------------
# Variation 1A: "The Monolith" (Faceted Crystalline Shard)
# A multi-faceted asymmetric titanium crystal shard with razor-sharp creases.
# Pure black & white, no circles, no letters.
# -------------------------------------------------------------------------
svg_1a = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg1a" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#090909"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border1a" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#383838" stop-opacity="0.9"/>
      <stop offset="100%" stop-color="#181818" stop-opacity="0.7"/>
    </linearGradient>
    <!-- Facet 1: Primary illuminated face -->
    <linearGradient id="f1a_main" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="60%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#a8a8a8"/>
    </linearGradient>
    <!-- Facet 2: Shaded secondary face -->
    <linearGradient id="f1a_side" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#8a8a8a"/>
      <stop offset="50%" stop-color="#545454"/>
      <stop offset="100%" stop-color="#2a2a2a"/>
    </linearGradient>
    <!-- Facet 3: Top architectural chamfer -->
    <linearGradient id="f1a_cap" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#cccccc"/>
    </linearGradient>
    <!-- Facet 4: Bottom anchor bevel -->
    <linearGradient id="f1a_bottom" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#404040"/>
      <stop offset="100%" stop-color="#181818"/>
    </linearGradient>
    <filter id="shadow1a" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg1a)" stroke="url(#border1a)" stroke-width="2"/>
  <rect x="25" y="25" width="462" height="462" rx="107" ry="107" fill="none" stroke="rgba(255, 255, 255, 0.05)" stroke-width="1"/>

  <g filter="url(#shadow1a)">
    <!-- Main Left Light Face -->
    <polygon points="256,108 152,324 236,404 256,108" fill="url(#f1a_main)"/>
    <!-- Right Shaded Face -->
    <polygon points="256,108 360,284 296,404 236,404" fill="url(#f1a_side)"/>
    <!-- Top Sharp Bevel Ridge -->
    <polygon points="256,108 300,164 256,182 212,164" fill="url(#f1a_cap)" opacity="0.95"/>
    <!-- Underside Shadow Plane -->
    <polygon points="236,404 296,404 266,420" fill="url(#f1a_bottom)"/>
  </g>
</svg>"""

with open("assets/shards/shard-1a-monolith.svg", "w", encoding="utf-8") as f:
    f.write(svg_1a)


# -------------------------------------------------------------------------
# Variation 1B: "The Twin Blades" / Sliced Negative-Space Core
# Two high-velocity diagonal aerodynamic blades passing with an ultra-clean slit.
# Represents speed, precision cut, zero bloat.
# -------------------------------------------------------------------------
svg_1b = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg1b" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#0a0a0a"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border1b" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#333333"/>
      <stop offset="100%" stop-color="#141414"/>
    </linearGradient>
    <!-- Left Blade -->
    <linearGradient id="blade_left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="70%" stop-color="#cccccc"/>
      <stop offset="100%" stop-color="#8a8a8a"/>
    </linearGradient>
    <!-- Right Blade -->
    <linearGradient id="blade_right" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#d4d4d4"/>
      <stop offset="70%" stop-color="#737373"/>
      <stop offset="100%" stop-color="#383838"/>
    </linearGradient>
    <filter id="shadow1b" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg1b)" stroke="url(#border1b)" stroke-width="2"/>

  <g filter="url(#shadow1b)">
    <!-- Primary Leading Blade (High & Sharp) -->
    <polygon points="174,386 238,126 302,126 238,386" fill="url(#blade_left)"/>
    <!-- Secondary Offset Blade (Stepped Down with precision slit) -->
    <polygon points="256,386 320,126 348,168 284,428" fill="url(#blade_right)"/>
  </g>
</svg>"""

with open("assets/shards/shard-1b-blades.svg", "w", encoding="utf-8") as f:
    f.write(svg_1b)


# -------------------------------------------------------------------------
# Variation 1C: "The Folded Ribbon" / Isometric Origami Prism
# Continuous twisted geometric plane in isometric 3D.
# High-end developer tool aesthetic (like Zed & Linear).
# -------------------------------------------------------------------------
svg_1c = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg1c" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#050505"/>
    </linearGradient>
    <linearGradient id="ribbon1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#cccccc"/>
    </linearGradient>
    <linearGradient id="ribbon2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#999999"/>
      <stop offset="100%" stop-color="#555555"/>
    </linearGradient>
    <linearGradient id="ribbon3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#444444"/>
      <stop offset="100%" stop-color="#1f1f1f"/>
    </linearGradient>
    <filter id="shadow1c" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="18" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg1c)" stroke="#2d2d2d" stroke-width="2"/>

  <g filter="url(#shadow1c)">
    <!-- Top Horizontal Plane -->
    <polygon points="152,192 272,126 360,176 240,242" fill="url(#ribbon1)"/>
    <!-- Left Vertical Drop Plane -->
    <polygon points="152,192 240,242 240,366 152,316" fill="url(#ribbon2)"/>
    <!-- Right Folded Return Plane -->
    <polygon points="240,242 360,176 360,300 240,366" fill="url(#ribbon3)"/>
  </g>
</svg>"""

with open("assets/shards/shard-1c-origami.svg", "w", encoding="utf-8") as f:
    f.write(svg_1c)


# -------------------------------------------------------------------------
# Render high-resolution PNGs (1024x1024) with Pillow
# -------------------------------------------------------------------------
def render_1a(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Points for Monolith Shard
    p_top = (int(s * 0.50), int(s * 0.21))
    p_left = (int(s * 0.30), int(s * 0.63))
    p_bottom = (int(s * 0.46), int(s * 0.79))
    p_right = (int(s * 0.70), int(s * 0.55))
    p_bottom_right = (int(s * 0.58), int(s * 0.79))

    # Right side (Darker metallic gray)
    draw.polygon([p_top, p_right, p_bottom_right, p_bottom], fill=(68, 68, 68, 255))
    draw.line([p_top, p_right, p_bottom_right, p_bottom], fill=(95, 95, 95, 255), width=int(s * 0.003))

    # Left side (Brilliant white)
    draw.polygon([p_top, p_left, p_bottom, p_top], fill=(235, 235, 235, 255))
    draw.line([p_top, p_left, p_bottom, p_top], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Specular spine line
    draw.line([p_top, p_bottom], fill=(255, 255, 255, 255), width=int(s * 0.006))

    # Top bevel cap
    p_cap_left = (int(s * 0.42), int(s * 0.32))
    p_cap_mid = (int(s * 0.50), int(s * 0.35))
    p_cap_right = (int(s * 0.58), int(s * 0.32))
    draw.polygon([p_top, p_cap_right, p_cap_mid, p_cap_left], fill=(255, 255, 255, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_1b(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Blade 1 (Left)
    b1 = [
        (int(s * 0.34), int(s * 0.75)),
        (int(s * 0.46), int(s * 0.25)),
        (int(s * 0.59), int(s * 0.25)),
        (int(s * 0.46), int(s * 0.75)),
    ]
    # Blade 2 (Right, stepped)
    b2 = [
        (int(s * 0.50), int(s * 0.75)),
        (int(s * 0.62), int(s * 0.25)),
        (int(s * 0.68), int(s * 0.33)),
        (int(s * 0.56), int(s * 0.83)),
    ]

    draw.polygon(b2, fill=(85, 85, 85, 255))
    draw.line(b2 + [b2[0]], fill=(125, 125, 125, 255), width=int(s * 0.003))

    draw.polygon(b1, fill=(240, 240, 240, 255))
    draw.line(b1 + [b1[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_1c(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Isometric Hexagon / Folded Ribbon
    p_left = (int(s * 0.30), int(s * 0.38))
    p_top = (int(s * 0.53), int(s * 0.25))
    p_right = (int(s * 0.70), int(s * 0.34))
    p_mid = (int(s * 0.47), int(s * 0.47))
    p_bottom = (int(s * 0.47), int(s * 0.71))
    p_left_bottom = (int(s * 0.30), int(s * 0.62))
    p_right_bottom = (int(s * 0.70), int(s * 0.58))

    # Right face (Darkest)
    draw.polygon([p_mid, p_right, p_right_bottom, p_bottom], fill=(55, 55, 55, 255))
    draw.line([p_mid, p_right, p_right_bottom, p_bottom], fill=(75, 75, 75, 255), width=int(s * 0.003))

    # Left face (Mid gray)
    draw.polygon([p_left, p_mid, p_bottom, p_left_bottom], fill=(145, 145, 145, 255))
    draw.line([p_left, p_mid, p_bottom, p_left_bottom], fill=(175, 175, 175, 255), width=int(s * 0.003))

    # Top face (Pure White Light Source)
    draw.polygon([p_left, p_top, p_right, p_mid], fill=(250, 250, 250, 255))
    draw.line([p_left, p_top, p_right, p_mid], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

if __name__ == "__main__":
    img_1a = render_1a(1024)
    img_1a.save("assets/shards/shard-1a-monolith.png")
    img_1a.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\shard_1a_monolith.png")

    img_1b = render_1b(1024)
    img_1b.save("assets/shards/shard-1b-blades.png")
    img_1b.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\shard_1b_blades.png")

    img_1c = render_1c(1024)
    img_1c.save("assets/shards/shard-1c-origami.png")
    img_1c.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\shard_1c_origami.png")

    print("Rendered 3 shard evolutions successfully!")

