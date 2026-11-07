"""Generate the application icon from simple vector shapes; Python stdlib only."""
from pathlib import Path
import math
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
SIZE, SCALE = 128, 3
BASE = (25, 23, 36)
ROSE = (235, 188, 186)
IRIS = (196, 167, 231)


def segment_distance(x, y, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = max(0, min(1, ((x-a[0])*dx + (y-a[1])*dy) / (dx*dx+dy*dy)))
    return math.hypot(x-a[0]-t*dx, y-a[1]-t*dy)


segments = [((20, 22), (32, 25)), ((32, 25), (44, 22)),
            ((20, 22), (20, 43)), ((44, 22), (44, 43)),
            ((20, 43), (32, 46)), ((32, 46), (44, 43)), ((32, 25), (32, 46)),
            ((24, 28), (28, 29)), ((36, 29), (40, 28))]


def pixel(x, y):
    d = math.hypot(max(abs(x-32)-20, 0), max(abs(y-32)-20, 0))
    if d > 12:
        return (0, 0, 0, 0)
    color = BASE
    if any(segment_distance(x, y, a, b) < .85 for a, b in segments):
        color = ROSE
    if segment_distance(x, y, (47, 14), (47, 21)) < .65 or segment_distance(x, y, (43.5, 17.5), (50.5, 17.5)) < .65:
        color = IRIS
    return (*color, 255)


raw = bytearray()
for y in range(SIZE):
    raw.append(0)
    for x in range(SIZE):
        samples = [pixel((x+(sx+.5)/SCALE)*64/SIZE, (y+(sy+.5)/SCALE)*64/SIZE)
                   for sy in range(SCALE) for sx in range(SCALE)]
        raw.extend(round(sum(p[c] for p in samples)/len(samples)) for c in range(4))


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind+data))


png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', SIZE, SIZE, 8, 6, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + chunk(b'IEND', b'')
(ROOT / 'assets/telysta.png').write_bytes(png)
ico = struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', SIZE, SIZE, 0, 0, 1, 32, len(png), 22) + png
(ROOT / 'assets/telysta.ico').write_bytes(ico)
print('Generated assets/telysta.png and assets/telysta.ico')
