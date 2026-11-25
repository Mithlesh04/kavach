// A growing list: push 20 million numbers, then add them up on all
// cores. Whole numbers add up to the same total in any order.
package main

import (
	"fmt"
	"runtime"
	"sync"
)

func main() {
	var items []int64
	for i := int64(0); i < 20000000; i++ {
		items = append(items, i%1000)
	}
	n := len(items)
	workers := runtime.NumCPU()
	parts := make([]int64, workers)
	var wg sync.WaitGroup
	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func(w int) {
			defer wg.Done()
			var part int64
			for _, x := range items[n*w/workers : n*(w+1)/workers] {
				part += x
			}
			parts[w] = part
		}(w)
	}
	wg.Wait()
	var total int64
	for _, part := range parts {
		total += part
	}
	fmt.Println(total)
}
