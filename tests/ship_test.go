package tests

import (
	"fmt"
	"testing"

	"spacemonger"
	"spacemonger/spacetraders"
)

func TestSortLocations(t *testing.T) {
	testLocations := []spacetraders.SystemLocation{{
		Symbol: "OE-PM",
		X:      -13,
		Y:      15,
	}, {
		Symbol: "OE-PM-TR",
		X:      -14,
		Y:      15,
	}, {
		Symbol: "OE-CR",
		X:      -8,
		Y:      12,
	}, {
		Symbol: "OE-KO",
		X:      1,
		Y:      50,
	}, {
		Symbol: "OE-UC",
		X:      45,
		Y:      -60,
	}, {
		Symbol: "OE-UC-AD",
		X:      44,
		Y:      -61,
	}, {
		Symbol: "OE-UC-OB",
		X:      48,
		Y:      -61,
	}, {
		Symbol: "OE-NY",
		X:      38,
		Y:      51,
	}, {
		Symbol: "OE-BO",
		X:      44,
		Y:      -71,
	}, {
		Symbol: "OE-W-XV",
		X:      18,
		Y:      101,
	}}

	bestCost, path:= spacemonger.SortLocations(testLocations)
	fmt.Printf("Best Cost: %f, Result path: %v\n", bestCost, path)

	// A→B→C→H→F→I→G→E→J→D→A
	// 0→1→2→7→5→8→6→4→9→3→0
	// fmt.Printf("Result: %+v\n", result)
}
