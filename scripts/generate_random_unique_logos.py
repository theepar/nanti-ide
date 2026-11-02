import math
import os
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("assets/random_unique", exist_ok=True)

# =========================================================================
# Concept 1: "The Hypercube / Tesseract" (Impossible 4D Geometry)
# An outer isometric hexagonal cube containing an inner suspended floating core
# with angled strut connectors. Extremely geek, cryptographic, high-IQ.
# =========================================================================
svg_hypercube = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg_hyp" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="hyp_top" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#cccccc"/>
    </linearGradient>
    <linearGradient id="hyp_left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#a8a8a8"/>
      <stop offset="100%" stop-color="#555555"/>
    </linearGradient>
    <linearGradient id="hyp_right" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#606060"/>
      <stop offset="100%" stop-color="#242424"/>
    </linearGradient>
    <filter id="sh_hyp" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg_hyp)" stroke="#333333" stroke-width="2"/>

  <g filter="url(#sh_hyp)">
    <!-- Outer Hexagonal Frame Planes -->
    <!-- Top Face -->
    <polygon points="256,110 380,182 256,254 132,182" fill="none" stroke="#666666" stroke-width="6"/>
    <!-- Left Face -->
    <polygon points="132,182 256,254 256,398 132,326" fill="none" stroke="#555555" stroke-width="6"/>
    <!-- Right Face -->
    <polygon points="256,254 380,182 380,326 256,398" fill="none" stroke="#444444" stroke-width="6"/>

    <!-- Inner Solid Suspended Core (Pure Solid Light) -->
    <polygon points="256,204 316,238 256,274 196,238" fill="url(#hyp_top)"/>
    <polygon points="196,238 256,274 256,344 196,308" fill="url(#hyp_left)"/>
    <polygon points="256,274 316,238 316,308 256,344" fill="url(#hyp_right)"/>

    <!-- 4D Hypercube Projection Struts -->
    <line x1="132" y1="182" x2="196" y2="238" stroke="#ffffff" stroke-width="5"/>
    <line x1="380" y1="182" x2="316" y2="238" stroke="#ffffff" stroke-width="5"/>
    <line x1="256" y1="110" x2="256" y2="204" stroke="#ffffff" stroke-width="5"/>
    <line x1="256" y1="398" x2="256" y2="344" stroke="#888888" stroke-width="5"/>
    <line x1="132" y1="326" x2="196" y2="308" stroke="#888888" stroke-width="5"/>
    <line x1="380" y1="326" x2="316" y2="308" stroke="#888888" stroke-width="5"/>
  </g>
</svg>"""

with open("assets/random_unique/random-hypercube.svg", "w", encoding="utf-8") as f:
    f.write(svg_hypercube)


# =========================================================================
# Concept 2: "The Kinetic Shuriken / Rotary Turbine" (Tri-Blade Rotor)
# 3 aerodynamic curved blades spiraling with razor-sharp rotational velocity.
# Looks intensely fast, lethal, cybernetic.
# =========================================================================
svg_shuriken = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg_shu" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="shu_b1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#8c8c8c"/>
    </linearGradient>
    <linearGradient id="shu_b2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#cccccc"/>
      <stop offset="100%" stop-color="#4d4d4d"/>
    </linearGradient>
    <linearGradient id="shu_b3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#999999"/>
      <stop offset="100%" stop-color="#262626"/>
    </linearGradient>
    <filter id="sh_shu" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg_shu)" stroke="#333333" stroke-width="2"/>

  <g filter="url(#sh_shu)">
    <!-- Blade 1 (Top North) -->
    <path d="M 256 256 L 256 108 C 298 126 350 162 368 214 L 292 232 Z" fill="url(#shu_b1)"/>
    <!-- Blade 2 (Southeast) -->
    <path d="M 256 256 L 384 330 C 362 372 316 414 260 418 L 256 342 Z" fill="url(#shu_b2)"/>
    <!-- Blade 3 (Southwest) -->
    <path d="M 256 256 L 128 330 C 114 278 126 218 162 174 L 222 226 Z" fill="url(#shu_b3)"/>
    
    <!-- Negative Space Center Hub Core -->
    <polygon points="256,236 273,266 239,266" fill="#0c0c0c"/>
  </g>
</svg>"""

with open("assets/random_unique/random-shuriken.svg", "w", encoding="utf-8") as f:
    f.write(svg_shuriken)


# =========================================================================
# Concept 3: "The Void Portal / Stepped Monolith Gateway"
# Minimalist brutalist architectural gateway. An ominous solid black & white
# structure with a stepped inner aperture cutting into eternity.
# =========================================================================
svg_monolith = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg_mon" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="pillar_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#a0a0a0"/>
    </linearGradient>
    <linearGradient id="pillar_r" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#888888"/>
      <stop offset="100%" stop-color="#333333"/>
    </linearGradient>
    <linearGradient id="lintel" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#cccccc"/>
    </linearGradient>
    <filter id="sh_mon" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="22" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg_mon)" stroke="#303030" stroke-width="2"/>

  <g filter="url(#sh_mon)">
    <!-- Heavy Architectural Lintel / Header -->
    <polygon points="132,130 380,130 356,182 156,182" fill="url(#lintel)"/>
    
    <!-- Left Staggered Gate Pillar -->
    <polygon points="156,182 224,182 208,382 140,382" fill="url(#pillar_l)"/>
    
    <!-- Right Staggered Gate Pillar -->
    <polygon points="288,182 356,182 372,382 304,382" fill="url(#pillar_r)"/>

    <!-- Central Floating Laser Horizon Bar -->
    <polygon points="200,282 312,282 304,298 208,298" fill="#ffffff"/>
  </g>
</svg>"""

with open("assets/random_unique/random-monolith.svg", "w", encoding="utf-8") as f:
    f.write(svg_monolith)


# =========================================================================
# Concept 4: "The Cybernetic Shield / Low-Poly Exoskeleton"
# A high-tech faceted armored exoskeleton shield plate with geometric
# ventilation gills. Powerful, indestructible native Rust metaphor.
# =========================================================================
svg_exoskeleton = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg_exo" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="exo_top_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#c8c8c8"/>
    </linearGradient>
    <linearGradient id="exo_top_r" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#999999"/>
      <stop offset="100%" stop-color="#4a4a4a"/>
    </linearGradient>
    <linearGradient id="exo_bot_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#808080"/>
    </linearGradient>
    <linearGradient id="exo_bot_r" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#666666"/>
      <stop offset="100%" stop-color="#262626"/>
    </linearGradient>
    <filter id="sh_exo" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="22" stdDeviation="24" flood-color="#000000" flood-opacity="0.95"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg_exo)" stroke="#383838" stroke-width="2"/>

  <g filter="url(#sh_exo)">
    <!-- Central Exoskeleton Chitin Plate -->
    <!-- Top Left Facet -->
    <polygon points="256,120 160,186 256,260" fill="url(#exo_top_l)"/>
    <!-- Top Right Facet -->
    <polygon points="256,120 352,186 256,260" fill="url(#exo_top_r)"/>
    <!-- Bottom Left Facet -->
    <polygon points="160,186 256,260 256,396 190,326" fill="url(#exo_bot_l)"/>
    <!-- Bottom Right Facet -->
    <polygon points="352,186 256,260 256,396 322,326" fill="url(#exo_bot_r)"/>

    <!-- Specular Center Spine Seam -->
    <line x1="256" y1="120" x2="256" y2="396" stroke="#ffffff" stroke-width="4"/>

    <!-- Cybernetic Slit Cutouts (Gills) -->
    <polygon points="144,220 180,244 172,256 136,232" fill="#ffffff"/>
    <polygon points="368,220 332,244 340,256 376,232" fill="#555555"/>
  </g>
</svg>"""

with open("assets/random_unique/random-exoskeleton.svg", "w", encoding="utf-8") as f:
    f.write(svg_exoskeleton)


# =========================================================================
# Render high-resolution PNGs (1024x1024)
# =========================================================================
def render_hypercube(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Outer Hexagon Points
    p_top = (int(s * 0.50), int(s * 0.21))
    p_tr = (int(s * 0.74), int(s * 0.35))
    p_br = (int(s * 0.74), int(s * 0.64))
    p_bot = (int(s * 0.50), int(s * 0.78))
    p_bl = (int(s * 0.26), int(s * 0.64))
    p_tl = (int(s * 0.26), int(s * 0.35))
    p_mid = (int(s * 0.50), int(s * 0.50))

    # Draw wireframe outer cube
    draw.line([p_top, p_tr, p_br, p_bot, p_bl, p_tl, p_top], fill=(90, 90, 90, 255), width=int(s * 0.008))
    draw.line([p_top, p_mid], fill=(110, 110, 110, 255), width=int(s * 0.008))
    draw.line([p_bot, p_mid], fill=(70, 70, 70, 255), width=int(s * 0.008))
    draw.line([p_tl, p_mid], fill=(90, 90, 90, 255), width=int(s * 0.008))
    draw.line([p_tr, p_mid], fill=(70, 70, 70, 255), width=int(s * 0.008))
    draw.line([p_bl, p_mid], fill=(70, 70, 70, 255), width=int(s * 0.008))
    draw.line([p_br, p_mid], fill=(50, 50, 50, 255), width=int(s * 0.008))

    # Inner Solid Floating Cube
    c_top = (int(s * 0.50), int(s * 0.40))
    c_tr = (int(s * 0.62), int(s * 0.47))
    c_br = (int(s * 0.62), int(s * 0.60))
    c_bot = (int(s * 0.50), int(s * 0.67))
    c_bl = (int(s * 0.38), int(s * 0.60))
    c_tl = (int(s * 0.38), int(s * 0.47))
    c_mid = (int(s * 0.50), int(s * 0.54))

    # Draw solid inner faces
    draw.polygon([c_mid, c_tr, c_br, c_bot], fill=(50, 50, 50, 255))
    draw.polygon([c_tl, c_mid, c_bot, c_bl], fill=(130, 130, 130, 255))
    draw.polygon([c_top, c_tr, c_mid, c_tl], fill=(250, 250, 250, 255))
    draw.line([c_top, c_tr, c_br, c_bot, c_bl, c_tl, c_top], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Connecting hyper-rays
    draw.line([p_top, c_top], fill=(255, 255, 255, 255), width=int(s * 0.008))
    draw.line([p_tr, c_tr], fill=(200, 200, 200, 255), width=int(s * 0.006))
    draw.line([p_tl, c_tl], fill=(200, 200, 200, 255), width=int(s * 0.006))
    draw.line([p_bot, c_bot], fill=(100, 100, 100, 255), width=int(s * 0.006))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_shuriken(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    cx, cy = s // 2, s // 2
    # Draw 3 rotary curved blades
    # Blade 1 (North)
    b1 = [(cx, cy), (cx, int(s * 0.21)), (int(s * 0.72), int(s * 0.42)), (int(s * 0.57), int(s * 0.45))]
    # Blade 2 (Southeast)
    b2 = [(cx, cy), (int(s * 0.75), int(s * 0.65)), (int(s * 0.51), int(s * 0.82)), (int(s * 0.50), int(s * 0.67))]
    # Blade 3 (Southwest)
    b3 = [(cx, cy), (int(s * 0.25), int(s * 0.65)), (int(s * 0.32), int(s * 0.34)), (int(s * 0.43), int(s * 0.44))]

    draw.polygon(b3, fill=(50, 50, 50, 255))
    draw.line(b3 + [b3[0]], fill=(80, 80, 80, 255), width=int(s * 0.003))

    draw.polygon(b2, fill=(110, 110, 110, 255))
    draw.line(b2 + [b2[0]], fill=(150, 150, 150, 255), width=int(s * 0.003))

    draw.polygon(b1, fill=(245, 245, 245, 255))
    draw.line(b1 + [b1[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Center triangular notch
    notch = [(cx, cy - int(s * 0.04)), (cx + int(s * 0.035), cy + int(s * 0.02)), (cx - int(s * 0.035), cy + int(s * 0.02))]
    draw.polygon(notch, fill=(10, 10, 10, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_monolith(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Architectural Gateway
    # Lintel
    lintel = [
        (int(s * 0.26), int(s * 0.25)),
        (int(s * 0.74), int(s * 0.25)),
        (int(s * 0.70), int(s * 0.35)),
        (int(s * 0.30), int(s * 0.35)),
    ]
    draw.polygon(lintel, fill=(250, 250, 250, 255))
    draw.line(lintel + [lintel[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Right Pillar (Shadow)
    p_r = [
        (int(s * 0.56), int(s * 0.35)),
        (int(s * 0.70), int(s * 0.35)),
        (int(s * 0.73), int(s * 0.75)),
        (int(s * 0.59), int(s * 0.75)),
    ]
    draw.polygon(p_r, fill=(65, 65, 65, 255))
    draw.line(p_r + [p_r[0]], fill=(95, 95, 95, 255), width=int(s * 0.003))

    # Left Pillar (Bright)
    p_l = [
        (int(s * 0.30), int(s * 0.35)),
        (int(s * 0.44), int(s * 0.35)),
        (int(s * 0.41), int(s * 0.75)),
        (int(s * 0.27), int(s * 0.75)),
    ]
    draw.polygon(p_l, fill=(195, 195, 195, 255))
    draw.line(p_l + [p_l[0]], fill=(235, 235, 235, 255), width=int(s * 0.003))

    # Horizon Bar
    bar = [
        (int(s * 0.39), int(s * 0.55)),
        (int(s * 0.61), int(s * 0.55)),
        (int(s * 0.59), int(s * 0.58)),
        (int(s * 0.41), int(s * 0.58)),
    ]
    draw.polygon(bar, fill=(255, 255, 255, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_exoskeleton(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Points for Armor Shield Plate
    p_top = (int(s * 0.50), int(s * 0.23))
    p_tl = (int(s * 0.31), int(s * 0.36))
    p_tr = (int(s * 0.69), int(s * 0.36))
    p_mid = (int(s * 0.50), int(s * 0.51))
    p_bot = (int(s * 0.50), int(s * 0.77))
    p_bl = (int(s * 0.37), int(s * 0.64))
    p_br = (int(s * 0.63), int(s * 0.64))

    # Bottom Right
    draw.polygon([p_mid, p_tr, p_br, p_bot], fill=(45, 45, 45, 255))
    draw.line([p_mid, p_tr, p_br, p_bot], fill=(70, 70, 70, 255), width=int(s * 0.003))

    # Top Right
    draw.polygon([p_top, p_tr, p_mid], fill=(105, 105, 105, 255))
    draw.line([p_top, p_tr, p_mid], fill=(140, 140, 140, 255), width=int(s * 0.003))

    # Bottom Left
    draw.polygon([p_tl, p_mid, p_bot, p_bl], fill=(185, 185, 185, 255))
    draw.line([p_tl, p_mid, p_bot, p_bl], fill=(225, 225, 225, 255), width=int(s * 0.003))

    # Top Left (Full Specular White)
    draw.polygon([p_top, p_mid, p_tl], fill=(255, 255, 255, 255))
    draw.line([p_top, p_mid, p_tl], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Center Spine
    draw.line([p_top, p_bot], fill=(255, 255, 255, 255), width=int(s * 0.006))

    return img.resize((size, size), Image.Resampling.LANCZOS)

if __name__ == "__main__":
    img_hyp = render_hypercube(1024)
    img_hyp.save("assets/random_unique/random-hypercube.png")
    img_hyp.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\random_hypercube.png")

    img_shu = render_shuriken(1024)
    img_shu.save("assets/random_unique/random-shuriken.png")
    img_shu.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\random_shuriken.png")

    img_mon = render_monolith(1024)
    img_mon.save("assets/random_unique/random-monolith.png")
    img_mon.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\random_monolith.png")

    img_exo = render_exoskeleton(1024)
    img_exo.save("assets/random_unique/random-exoskeleton.png")
    img_exo.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\random_exoskeleton.png")

    print("Rendered 4 wild random concepts successfully!")
