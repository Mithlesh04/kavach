class Order {
  constructor(item, qty, cents) {
    this.item = item;
    this.qty = qty;
    this.cents = cents;
  }

  cost() {
    return this.qty * this.cents;
  }
}

class BadQty extends Error {
  constructor(item) {
    super(`no quantity for ${item}`);
    this.item = item;
  }
}

function total(orders) {
  for (const o of orders) {
    if (o.qty <= 0) {
      throw new BadQty(o.item);
    }
  }
  return orders.reduce((sum, o) => sum + o.cost(), 0);
}

const codes = new Map([["SPRING", 10], ["VIP", 25]]);
const orders = [new Order("pen", 3, 150), new Order("cup", 1, 900)];

let sum;
try {
  sum = total(orders);
} catch {
  sum = 0;
}
const off = codes.get("SPRING") ?? 0;
console.log(`total ${sum}, with ${off}% off ${Math.trunc((sum * (100 - off)) / 100)}`);
console.log(`WINTER gives ${codes.get("WINTER") ?? 0}%`);

const broken = [new Order("kite", 0, 500)];
try {
  console.log(`total ${total(broken)}`);
} catch (e) {
  if (!(e instanceof BadQty)) throw e;
  console.log(`refused: no quantity for ${e.item}`);
}
