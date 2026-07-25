#!/usr/bin/env python3

import json
from pathlib import Path


ASSETS_DIR = Path("src/Assets.xcassets")
INFO = {"author": "xcode", "version": 1}
IMAGESETS = ("green", "red", "yellow")
APPICON_SLOTS = (
    ("16x16", "1x", None),
    ("16x16", "2x", None),
    ("32x32", "1x", "icon-32x32.png"),
    ("32x32", "2x", None),
    ("128x128", "1x", None),
    ("128x128", "2x", None),
    ("256x256", "1x", None),
    ("256x256", "2x", None),
    ("512x512", "1x", None),
    ("512x512", "2x", None),
)


def write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def imageset_contents(name: str):
    return {
        "images": [
            {"filename": f"{name}.png", "idiom": "universal", "scale": "1x"},
            {"filename": f"{name}@2x.png", "idiom": "universal", "scale": "2x"},
            {"filename": f"{name}@3x.png", "idiom": "universal", "scale": "3x"},
        ],
        "info": INFO,
    }


def appicon_contents():
    images = []
    for size, scale, filename in APPICON_SLOTS:
        image = {"idiom": "mac", "scale": scale, "size": size}
        if filename is not None:
            image["filename"] = filename
        images.append(image)
    return {"images": images, "info": INFO}


def main() -> None:
    write_json(ASSETS_DIR / "Contents.json", {"info": INFO})
    write_json(ASSETS_DIR / "AppIcon.appiconset" / "Contents.json", appicon_contents())

    for name in IMAGESETS:
        write_json(ASSETS_DIR / f"{name}.imageset" / "Contents.json", imageset_contents(name))


if __name__ == "__main__":
    main()
