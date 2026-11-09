// Single reads from a large map: 20 million lookups at keys a Lehmer
// generator picks, over 2 million entries.
function main() {
    const m = new Map();
    for (let i = 0; i < 2_000_000; i++) {
        m.set(i, i * 2 + 1);
    }
    let j = 12345;
    let total = 0;
    for (let r = 0; r < 20_000_000; r++) {
        j = (j * 48271) % 2147483647;
        const v = m.get(j % 2_000_000);
        total += v === undefined ? 0 : v;
    }
    console.log(total);
}

main();
