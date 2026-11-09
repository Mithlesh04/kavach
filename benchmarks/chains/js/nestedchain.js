// A chain inside a chain: ten thousand rows, each with its own chain.
const rows = [];
for (let r = 0; r < 10_000; r++) {
  const row = new Array(1000);
  for (let i = 0; i < 1000; i++) row[i] = (r + i) % 1000;
  rows.push(row);
}
const total = rows.reduce(
  (a, row) =>
    a +
    row
      .map((x) => x * 2)
      .filter((x) => x % 3 !== 0)
      .reduce((b, x) => b + x, 0),
  0,
);
console.log(total);
