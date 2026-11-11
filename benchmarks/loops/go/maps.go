// A map with 2 million keys: insert, then look every key up.
package main

import "fmt"

func main() {
	squares := map[int64]int64{}
	for i := int64(0); i < 2000000; i++ {
		squares[i] = i * i % 1000
	}
	var total int64
	for i := int64(0); i < 2000000; i++ {
		if v, ok := squares[i]; ok {
			total += v
		}
	}
	fmt.Println(total)
}
