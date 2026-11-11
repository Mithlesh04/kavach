// Float arithmetic in a loop: 200 million rounds.
package main

import "fmt"

func main() {
	x, y := 0.0, 1.0
	for i := int64(0); i < 200000000; i++ {
		x = x*0.999999 + y
		y = y*1.0000001 - 0.0000001
	}
	fmt.Println(x > 0.0)
}
