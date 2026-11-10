export interface Shape {
  area(): number;
  readonly name: string;
}

export enum Mode {
  Fast,
  Slow,
}

export const pick = (items: number[], mode: Mode): number | undefined => {
  for (const item of items) {
    if (mode === Mode.Fast && item > 0) {
      return item;
    }
  }
  return items.map((value) => (value > 0 ? value : 0))[0];
};

export class Square implements Shape {
  readonly name = "square";
  private side: number;

  constructor(side: number) {
    this.side = side;
  }

  area(): number {
    try {
      return this.side * this.side;
    } catch (error) {
      return 0;
    }
  }

  scale = (factor: number) => this.side * factor;
}

const twice = x => x * 2;
