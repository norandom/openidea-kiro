import sys
from pathlib import Path

# Ensure src is in Python module search path
_src = Path(__file__).parent / "src"
if str(_src) not in sys.path:
  sys.path.insert(0, str(_src))

from kiro.main import Kiro

__all__ = ["Kiro"]
