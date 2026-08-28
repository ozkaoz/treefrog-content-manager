import pathlib

MUSIC_EXTS = {".mp3",".m4a",".aac",".wav",".flac",".ogg",".opus"}
VIDEO_EXTS = {".mp4",".mkv",".avi",".mov",".m4v",".wmv",".mpg",".mpeg",".ts",".webm"}
IMAGE_EXTS = {".jpg",".jpeg",".png",".bmp",".gif",".webp",".tiff",".tif",".tga",".ico"}
EBOOK_EXTS = {".epub",".mobi",".pdf",".cbz",".fb2",".xps"}
ARCHIVE_EXTS = {".zip",".7z",".rar"}
BIOS_HINTS = ["scph","gba_bios.bin","o2rom.bin","disksys.rom","neogeo.zip","bios_cd","kick13.rom","kick20.rom","pcfx.rom","x86boot.img"]

def classify(path: pathlib.Path, profile):
    ext = path.suffix.lower()
    name_lower = path.name.lower()
    # archives
    if ext in ARCHIVE_EXTS:
        return {"kind":"archive","system_id":None,"destination":"","multi_file":False,"archive_valid":False}
    if ext in MUSIC_EXTS:
        return {"kind":"music","system_id":None,"destination":"roms/music","multi_file":False,"archive_valid":False}
    if ext in VIDEO_EXTS:
        return {"kind":"video","system_id":None,"destination":"roms/videos","multi_file":False,"archive_valid":False}
    if ext in IMAGE_EXTS:
        # .res artwork
        if ".res" in [p.name for p in path.parents] or path.parent.name in (".res","Imgs","images","Images"):
            return {"kind":"image","system_id":None,"destination":".res","multi_file":False,"archive_valid":False}
        return {"kind":"image","system_id":None,"destination":"roms/images","multi_file":False,"archive_valid":False}
    if ext in EBOOK_EXTS:
        return {"kind":"ebook","system_id":None,"destination":"roms/Ebook","multi_file":False,"archive_valid":False}
    # LGPT
    if "lgpt" in str(path).lower() and ext in (".wav",".flac",".aiff"):
        return {"kind":"lgpt_sample","system_id":None,"destination":"lgpt/samples","multi_file":False,"archive_valid":False}
    if ext == ".lgpt" or ("projects" in str(path).lower() and path.is_dir() if hasattr(path,"is_dir") else False):
        return {"kind":"lgpt_project","system_id":None,"destination":"lgpt/projects","multi_file":True,"archive_valid":False}
    # BIOS by name hints
    for pat in BIOS_HINTS:
        if pat in name_lower:
            return {"kind":"bios","system_id":None,"destination":"cubegm/bios","multi_file":False,"archive_valid":False}
    # ROM by profile
    ext_to_system = profile["ext_to_system"]
    if ext in ext_to_system:
        sys_id = ext_to_system[ext][0]
        sys_entry = profile["sys_by_id"].get(sys_id, {})
        folder = sys_entry.get("folder_aliases", ["UNKNOWN"])[0]
        dest = f"roms/{folder}"
        multi = bool(sys_entry.get("multi_file", False))
        archive_valid = ext in [e.lower() for e in sys_entry.get("archive_payload_valid",[])]
        return {"kind":"rom","system_id":sys_id,"destination":dest,"multi_file":multi,"archive_valid":archive_valid}
    # unknown
    return {"kind":"unknown","system_id":None,"destination":"roms/UNKNOWN","multi_file":False,"archive_valid":False}
