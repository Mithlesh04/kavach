// Sorting 5 million numbers on all cores. As Kavach's sort does, one
// pass splits the numbers by their top 8 bits into 256 buckets, then
// the buckets are sorted on all cores (here with slices.Sort). The
// numbers are never negative.
package main

import (
	"fmt"
	"math/bits"
	"runtime"
	"slices"
	"sync"
	"sync/atomic"
)

func sortOnCores(v []int64) []int64 {
	var all int64
	for _, x := range v {
		all |= x
	}
	shift := bits.Len64(uint64(all)|1) - 8
	if shift < 0 {
		shift = 0
	}
	var start [257]int
	for _, x := range v {
		start[(x>>shift)+1]++
	}
	for b := 0; b < 256; b++ {
		start[b+1] += start[b]
	}
	out := make([]int64, len(v))
	var at [256]int
	copy(at[:], start[:256])
	for _, x := range v {
		out[at[x>>shift]] = x
		at[x>>shift]++
	}
	var next atomic.Int64
	var wg sync.WaitGroup
	for w := 0; w < runtime.NumCPU(); w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for b := int(next.Add(1) - 1); b < 256; b = int(next.Add(1) - 1) {
				slices.Sort(out[start[b]:start[b+1]])
			}
		}()
	}
	wg.Wait()
	return out
}

func main() {
	var items []int64
	x := int64(12345)
	for i := int64(0); i < 5000000; i++ {
		x = (x*1103515245 + 12345) % 2147483648
		items = append(items, x)
	}
	sorted := sortOnCores(slices.Clone(items))
	fmt.Println(sorted[0] + sorted[4999999])
}
