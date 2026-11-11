// A tight `for` loop over integers: 500 million rounds.
package main

import "fmt"

func main() {
	var total int64
	for i := int64(0); i < 500000000; i++ {
		total = (total + i*7%13) % 1000000007
	}
	fmt.Println(total)
}
