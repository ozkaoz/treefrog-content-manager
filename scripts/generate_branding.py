#!/usr/bin/env python3
"""
Generate TreeFrog frog-only branding from xgame-logo.bmp (TreeFrogUI upstream).
Deterministic, no redraw.
"""
from PIL import Image
from pathlib import Path
import urllib.request, ssl, os

ssl._create_default_https_context = ssl._create_unverified_context

REPO_ROOT = Path(__file__).resolve().parents[1]
TEMP = Path(os.environ.get("TEMP", "/tmp")) / "opencode_branding"
TEMP.mkdir(exist_ok=True)
SRC_URL = "https://raw.githubusercontent.com/tzubertowski/TreeFrogUI/main/xgame-logo.bmp"
SRC = TEMP / "xgame-logo.bmp"

def fetch():
    if not SRC.exists():
        print(f"fetch {SRC_URL}")
        data = urllib.request.urlopen(SRC_URL, timeout=30).read()
        SRC.write_bytes(data)
    else:
        print(f"using cached {SRC} {SRC.stat().st_size} bytes")

def is_bg(r,g,b): return r<20 and g<20 and b<20

def main():
    fetch()
    im = Image.open(SRC).convert("RGBA")
    w,h = im.size
    print(f"source {w}x{h}")
    # overall bbox
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
    print(f"overall bbox {min_x},{min_y},{max_x},{max_y} {max_x-min_x+1}x{max_y-min_y+1}")
    counts=[]
    for y in range(min_y, max_y+1):
        cnt=sum(1 for x in range(min_x, max_x+1) if not is_bg(*im.getpixel((x,y))[:3]))
        counts.append((y,cnt))
    # longest gap
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
    print(f"gap {best_gap} len {best_len}")
    split_y=(best_gap[0]+best_gap[1])//2 if best_gap else (min_y+max_y)//2
    frog_bbox=(min_x, min_y, max_x, split_y)
    frog=im.crop(frog_bbox)
    frog2=Image.new("RGBA", frog.size, (0,0,0,0))
    for y in range(frog.size[1]):
        for x in range(frog.size[0]):
            r,g,b,a=frog.getpixel((x,y))
            if is_bg(r,g,b):
                frog2.putpixel((x,y),(0,0,0,0))
            else:
                frog2.putpixel((x,y),(r,g,b,255))
    bbox=frog2.getbbox()
    trimmed=frog2.crop(bbox) if bbox else frog2
    print(f"trimmed {trimmed.size}")
    # square
    max_side=max(trimmed.size)
    square=Image.new("RGBA",(max_side,max_side),(0,0,0,0))
    square.paste(trimmed, ((max_side-trimmed.size[0])//2, (max_side-trimmed.size[1])//2))
    print(f"square {square.size}")

    out_branding=REPO_ROOT/"treefrog-manager"/"src"/"assets"/"branding"
    out_branding.mkdir(parents=True, exist_ok=True)
    trimmed.save(out_branding/"frog-only.png")
    square.save(out_branding/"frog-square.png")
    print(f"saved branding {out_branding}")

    out_icons=REPO_ROOT/"treefrog-manager"/"src-tauri"/"icons"
    out_icons.mkdir(parents=True, exist_ok=True)
    def save_resize(img,size,dest):
        im2=img.resize((size,size), Image.NEAREST)
        im2.save(dest, "PNG")
        print(f"saved {dest} {size}")

    save_resize(square,32,out_icons/"32x32.png")
    save_resize(square,128,out_icons/"128x128.png")
    save_resize(square,256,out_icons/"128x128@2x.png")
    save_resize(square,256,out_icons/"256x256.png")
    save_resize(square,512,out_icons/"512x512.png")
    ico_sizes=[16,32,48,256]
    ico_imgs=[square.resize((s,s), Image.NEAREST) for s in ico_sizes]
    ico_imgs[0].save(out_icons/"icon.ico", sizes=[(s,s) for s in ico_sizes])
    print(f"saved ico")
    try:
        square.resize((512,512), Image.NEAREST).save(out_icons/"icon.icns", format="ICNS")
        print("saved icns")
    except Exception as e:
        print("icns fallback",e)
        square.resize((512,512), Image.NEAREST).save(out_icons/"512x512.png")
        import shutil; shutil.copy(out_icons/"512x512.png", out_icons/"icon.icns")

if __name__=="__main__":
    main()
