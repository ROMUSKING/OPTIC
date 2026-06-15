#!/usr/bin/env python3
from pathlib import Path
import json, sys
root = Path(__file__).resolve().parent
manifest = json.loads((root / 'manifest.json').read_text())
out = ''.join((root / rel).read_text() for rel in manifest['order'])
target = Path(sys.argv[1]) if len(sys.argv) > 1 else root / 'assembled.md'
target.write_text(out)
print(target)
