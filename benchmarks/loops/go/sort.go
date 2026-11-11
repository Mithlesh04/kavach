// Sorting 5 million numbers.
package main

import (
	"fmt"
	"slices"
)

func main() {
	var items []int64
	x := int64(12345)
	for i := int64(0); i < 5000000; i++ {
		x = (x*1103515245 + 12345) % 2147483648
		items = append(items, x)
	}
	sorted := slices.Clone(items)
	slices.Sort(sorted)
	fmt.Println(sorted[0] + sorted[4999999])
}
