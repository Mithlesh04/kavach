// Single reads from a large list: 50 million lookups at indexes a
// Lehmer generator picks, over 10 million items.
function main() {
    const xs = [];
    for (let i = 0; i < 10_000_000; i++) {
        xs.push(i * 2);
    }
    let j = 12345;
    let total = 0;
    for (let r = 0; r < 50_000_000; r++) {
        j = (j * 48271) % 2147483647;
        total += xs[j % 10_000_000];
    }
    console.log(total);
}

main();
