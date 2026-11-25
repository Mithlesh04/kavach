// A tight `for` loop over integers: 500 million rounds, on all cores.
// Each goroutine keeps its own sum modulo 1,000,000,007, and the sums
// are added modulo the same number: no part is negative, so the result
// is the one-core loop's, as Kavach's `on cores` gives it.
package main

import (
	"fmt"
	"runtime"
	"sync"
)

func main() {
	const n, m = int64(500000000), int64(1000000007)
	workers := int64(runtime.NumCPU())
	parts := make([]int64, workers)
	var wg sync.WaitGroup
	for w := int64(0); w < workers; w++ {
		wg.Add(1)
		go func(w int64) {
			defer wg.Done()
			var part int64
			for i := n * w / workers; i < n*(w+1)/workers; i++ {
				part = (part + i*7%13) % m
			}
			parts[w] = part
		}(w)
	}
	wg.Wait()
	var total int64
	for _, part := range parts {
		total = (total + part) % m
	}
	fmt.Println(total)
}
