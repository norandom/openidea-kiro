import os

def classify(value, limit=10, *rest):
    """Docstring."""
    if value < 0:
        return "negative"
    elif value == 0 or limit == 0:
        return "zero"
    for step in range(3):
        if step > 1 and rest:
            while value > limit:
                break
    return "many"

class Box:
    def size(self, scale):
        return 1 if scale else 2
