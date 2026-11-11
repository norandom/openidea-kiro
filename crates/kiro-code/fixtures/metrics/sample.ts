export function classify(value: number, limit = 10): string {
  if (value < 0) {
    return "negative";
  } else if (value === 0 || limit === 0) {
    return "zero";
  }
  for (let step = 0; step < 3; step++) {
    if (step > 1 && limit) {
      while (value > limit) {
        break;
      }
    }
  }
  switch (value) {
    case 1:
      return "one";
    default:
      return value > 5 ? "many" : "few";
  }
}

class Box {
  size(scale: number): number {
    return scale ?? 1;
  }
}
