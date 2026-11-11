// Records: create 10 million points in a list, then read their fields.
package main

import "fmt"

type Point struct {
	x, y int64
}

func main() {
	var points []Point
	for i := int64(0); i < 10000000; i++ {
		points = append(points, Point{i % 100, i % 7})
	}
	var total int64
	for _, p := range points {
		total += p.x * p.y
	}
	fmt.Println(total)
}
