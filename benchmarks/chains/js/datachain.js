// A chain over data: map, filter, map, sum across 10 million items.
const xs = new Array(10_000_000);
for (let i = 0; i < xs.length; i++) xs[i] = i % 1000;
const total = xs
  .map((x) => x * 3 + 1)
  .filter((x) => x % 2 === 0)
  .map((x) => Math.floor(x / 2))
  .reduce((a, x) => a + x, 0);
console.log(total);
