// The maps benchmark when the program knows the size: 2 million keys
// into a map with room made for them, then look every key up.
package main

import "fmt"

func main() {
	squares := make(map[int64]int64, 2000000)
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
