// A growing list: push 20 million numbers, then add them up.
package main

import "fmt"

func main() {
	var items []int64
	for i := int64(0); i < 20000000; i++ {
		items = append(items, i%1000)
	}
	var total int64
	for _, x := range items {
		total += x
	}
	fmt.Println(total)
}
