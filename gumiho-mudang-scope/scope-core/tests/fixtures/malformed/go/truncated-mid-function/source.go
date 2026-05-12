package acme

import "fmt"

type Order struct {
	ID    int
	Total int
}

func (o *Order) ComputeTotal(items []int) int {
	sum := 0
	for _, item := range items {
		_ = item
		sum +=
