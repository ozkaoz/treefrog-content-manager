#!/usr/bin/env python3
"""
Generate TreeFrog frog-only branding — corrected orientation & high-res.
Canonical source: logo.png (1536x1024, TreeFrogUI) — frog left, wordmark right,
upright for desktop README. xgame-logo.bmp (480x854) is vertical for handheld
boot and is stored top-down but appears inverted on desktop due to handheld
display rotation; using logo.png avoids inversion and gives higher-res frog.

Deterministic, no redraw, NEAREST for pixel-art.
"""
from PIL import Image
from pathlib import Path
import urllib.request, ssl, os

ssl._create_default_https_context = ssl._create_unverified_context

REPO_ROOT = Path(__file__).resolve().parents[1]
TEMP = Path(os.environ.get("TEMP", "/tmp")) / "opencode_branding"
TEMP.mkdir(exist_ok=True)

# Prefer logo.png (horizontal, desktop upright, 1536x1024, high-res)
SRC_URL_LOGO = "https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/logo.png"
SRC_LOGO = TEMP / "logo.png"
SRC_URL_XGAME = "https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/xgame-logo.bmp"
SRC_XGAME = TEMP / "xgame-logo.bmp"

def fetch():
    for url, path in [(SRC_URL_LOGO, SRC_LOGO), (SRC_URL_XGAME, SRC_XGAME)]:
        if not path.exists():
            print(f"fetch {url}")
            try:
                data = urllib.request.urlopen(url, timeout=30).read()
                path.write_bytes(data)
                print(f"  saved {path} {len(data)} bytes")
            except Exception as e:
                print(f"  fetch failed {url}: {e}")
        else:
            print(f"using cached {path} {path.stat().st_size} bytes")

def is_bg(r,g,b): return r<20 and g<20 and b<20

def extract_frog_logo():
    """Extract frog from logo.png (horizontal): left of x-gap."""
    im = Image.open(SRC_LOGO).convert("RGBA")
    w,h = im.size
    print(f"logo source {w}x{h}")
    min_x, max_x = w, 0
    min_y, max_y = h, 0
    for y in range(h):
        for x in range(w):
            r,g,b,a = im.getpixel((x,y))
            if not is_bg(r,g,b):
                if x<min_x: min_x=x
                if x>max_x: max_x=x
                if y<min_y: min_y=y
                if y>max_y: max_y=y
    print(f"logo overall bbox {min_x},{min_y},{max_x},{max_y} {max_x-min_x+1}x{max_y-min_y+1}")
    # gap in x between frog (left) and wordmark (right)
    counts=[]
    for x in range(min_x, max_x+1):
        cnt=sum(1 for y in range(min_y, max_y+1) if not is_bg(*im.getpixel((x,y))[:3]))
        counts.append((x,cnt))
    best_gap=None; best_len=0; cur_start=None; cur_len=0
    for x,cnt in counts:
        if cnt<5:
            if cur_start is None:
                cur_start=x; cur_len=1
            else: cur_len+=1
        else:
            if cur_len>best_len:
                best_len=cur_len; best_gap=(cur_start, x-1)
            cur_start=None; cur_len=0
    if cur_len>best_len:
        best_len=cur_len; best_gap=(cur_start, max_x)
    print(f"logo gap x {best_gap} len {best_len}")
    split_x=(best_gap[0]+best_gap[1])//2 if best_gap else (min_x+max_x)//2
    frog_bbox=(min_x, min_y, split_x, max_y)
    frog=im.crop(frog_bbox)
    print(f"frog_logo bbox {frog_bbox} {frog.size}")
    # make transparent
    w2,h2=frog.size
    out=Image.new("RGBA", frog.size, (0,0,0,0))
    for y in range(h2):
        for x in range(w2):
            r,g,b,a=frog.getpixel((x,y))
            if is_bg(r,g,b):
                out.putpixel((x,y),(0,0,0,0))
            else:
                out.putpixel((x,y),(r,g,b,255))
    bbox=out.getbbox()
    trimmed=out.crop(bbox) if bbox else out
    print(f"trimmed logo frog {trimmed.size} (no flip — logo.png is desktop upright)")
    return trimmed

def extract_frog_xgame_flipped():
    """Fallback: xgame frog flipped vertically (handheld boot stored inverted)."""
    im = Image.open(SRC_XGAME).convert("RGBA")
    w,h = im.size
    print(f"xgame source {w}x{h}")
    min_x, max_x = w, 0
    min_y, max_y = h, 0
    for y in range(h):
        for x in range(w):
            r,g,b,a = im.getpixel((x,y))
            if not is_bg(r,g,b):
                if x<min_x: min_x=x
                if x>max_x: max_x=x
                if y<min_y: min_y=y
                if y>max_y: max_y=y
    print(f"xgame overall bbox {min_x},{min_y},{max_x},{max_y}")
    counts=[]
    for y in range(min_y, max_y+1):
        cnt=sum(1 for x in range(min_x, max_x+1) if not is_bg(*im.getpixel((x,y))[:3]))
        counts.append((y,cnt))
    best_gap=None; best_len=0; cur_start=None; cur_len=0
    for y,cnt in counts:
        if cnt<5:
            if cur_start is None:
                cur_start=y; cur_len=1
            else: cur_len+=1
        else:
            if cur_len>best_len:
                best_len=cur_len; best_gap=(cur_start, y-1)
            cur_start=None; cur_len=0
    if cur_len>best_len:
        best_len=cur_len; best_gap=(cur_start, max_y)
    print(f"xgame gap y {best_gap} len {best_len}")
    split_y=(best_gap[0]+best_gap[1])//2 if best_gap else (min_y+max_y)//2
    frog_bbox=(min_x, min_y, max_x, split_y)
    frog=im.crop(frog_bbox)
    print(f"frog xgame bbox {frog_bbox} {frog.size}")
    w2,h2=frog.size
    out=Image.new("RGBA", frog.size, (0,0,0,0))
    for y in range(h2):
        for x in range(w2):
            r,g,b,a=frog.getpixel((x,y))
            if is_bg(r,g,b):
                out.putpixel((x,y),(0,0,0,0))
            else:
                out.putpixel((x,y),(r,g,b,255))
    bbox=out.getbbox()
    trimmed=out.crop(bbox) if bbox else out
    print(f"trimmed xgame frog {trimmed.size} before flip")
    # ROOT CAUSE: xgame frog appears inverted on desktop because handheld boot
    # expects inverted storage for rotated display; flip vertically for desktop
    trimmed_flipped = trimmed.transpose(Image.FLIP_TOP_BOTTOM)
    print(f"trimmed xgame flipped {trimmed_flipped.size}")
    return trimmed_flipped

def main():
    fetch()
    # Primary: logo.png frog (high-res, desktop upright, no flip needed)
    # If logo fetch failed, fallback to xgame flipped
    try:
        trimmed = extract_frog_logo()
        source_note = "logo.png (1536x1024, TreeFrogUI, desktop upright, high-res)"
    except Exception as e:
        print(f"logo extraction failed {e}, fallback to xgame flipped")
        trimmed = extract_frog_xgame_flipped()
        source_note = "xgame-logo.bmp (480x854, flipped for desktop)"

    print(f"canonical trimmed {trimmed.size} from {source_note}")

    # Also keep xgame flipped reference for comparison (not committed)
    try:
        _xgame_ref = extract_frog_xgame_flipped()
        print(f"xgame reference { _xgame_ref.size } (for documentation)")
    except: pass

    # square padded (centered, transparent)
    max_side=max(trimmed.size)
    # Use square size as max_side, but ensure at least 512 for icons
    square=Image.new("RGBA",(max_side,max_side),(0,0,0,0))
    square.paste(trimmed, ((max_side-trimmed.size[0])//2, (max_side-trimmed.size[1])//2))
    print(f"square {square.size}")

    out_branding=REPO_ROOT/"treefrog-manager"/"src"/"assets"/"branding"
    out_branding.mkdir(parents=True, exist_ok=True)
    trimmed.save(out_branding/"frog-only.png")
    square.save(out_branding/"frog-square.png")
    print(f"saved branding {out_branding} frog-only {trimmed.size} square {square.size}")
    # Document source
    readme = out_branding/"README.md"
    if readme.exists():
        txt = readme.read_text(encoding="utf-8")
        # Ensure it mentions correct orientation fix
        if "FLIP" not in txt:
            print("README already exists, not overwriting orientation note — update manually if needed")

    out_icons=REPO_ROOT/"treefrog-manager"/"src-tauri"/"icons"
    out_icons.mkdir(parents=True, exist_ok=True)
    def save_resize(img,size,dest):
        im2=img.resize((size,size), Image.NEAREST)
        im2.save(dest, "PNG")
        print(f"saved {dest} {size}")

    # Use high-res square for icons; ensure square is at least 512 for best quality
    # If square is smaller than 512, upscale via NEAREST to 512 first
    base_for_icons = square
    if max_side < 512:
        # Upscale square to 512 via NEAREST for icon generation (keeps pixel-art crisp)
        base_for_icons = square.resize((512,512), Image.NEAREST)
        print(f"upscaled base for icons to 512")

    save_resize(base_for_icons,32,out_icons/"32x32.png")
    save_resize(base_for_icons,64,out_icons/"64x64.png")
    save_resize(base_for_icons,128,out_icons/"128x128.png")
    save_resize(base_for_icons,256,out_icons/"128x128@2x.png")
    save_resize(base_for_icons,256,out_icons/"256x256.png")
    save_resize(base_for_icons,512,out_icons/"512x512.png")

    # ICO: use largest image as source, save with multiple sizes via sizes param
    # Pillow will generate ICO with PNG-compressed entries for 256
    ico_path = out_icons/"icon.ico"
    # Use 256 PNG as source for ICO to ensure all sizes are generated correctly
    icon_256 = base_for_icons.resize((256,256), Image.NEAREST)
    # Save with sizes including 16,32,48,64,256
    icon_256.save(ico_path, sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
    print(f"saved ico {ico_path} {ico_path.stat().st_size} bytes (should be >5k)")

    # ICNS: 512
    icns_path = out_icons/"icon.icns"
    try:
        base_for_icons.resize((512,512), Image.NEAREST).save(icns_path, format="ICNS")
        print(f"saved icns {icns_path} {icns_path.stat().st_size} bytes")
    except Exception as e:
        print(f"icns fallback {e}")
        base_for_icons.resize((512,512), Image.NEAREST).save(out_icons/"512x512.png")
        import shutil; shutil.copy(out_icons/"512x512.png", icns_path)

if __name__=="__main__":
    main()
