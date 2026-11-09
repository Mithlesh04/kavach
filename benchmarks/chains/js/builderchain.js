// A chain of builder methods, 10 million times.
class Transfer {
  constructor(from = 0, to = 0, amount = 0, fee = 0, note = 0) {
    this.from = from;
    this.to = to;
    this.amount = amount;
    this.fee = fee;
    this.note = note;
  }
  fromAccount(a) {
    return new Transfer(a, this.to, this.amount, this.fee, this.note);
  }
  toAccount(a) {
    return new Transfer(this.from, a, this.amount, this.fee, this.note);
  }
  pay(n) {
    return new Transfer(this.from, this.to, n, this.fee, this.note);
  }
  withFee(n) {
    return new Transfer(this.from, this.to, this.amount, n, this.note);
  }
  tagged(n) {
    return new Transfer(this.from, this.to, this.amount, this.fee, n);
  }
}

let total = 0;
for (let i = 0; i < 10_000_000; i++) {
  const t = new Transfer()
    .fromAccount(i)
    .toAccount(i + 1)
    .pay(i % 100)
    .withFee(i % 7)
    .tagged(i % 3);
  total += t.amount + t.fee + t.note;
}
console.log(total);
