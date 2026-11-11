// Lambdas over a big list: map then filter then sum, 10 million items.
package main

import "fmt"

func main() {
	var items []int64
	for i := int64(0); i < 10000000; i++ {
		items = append(items, i%1000)
	}
	var doubled []int64
	for _, x := range items {
		doubled = append(doubled, x*2)
	}
	var big []int64
	for _, x := range doubled {
		if x > 1000 {
			big = append(big, x)
		}
	}
	var total int64
	for _, x := range big {
		total += x
	}
	fmt.Println(total)
}
