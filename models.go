package spacemonger

import (
	"spacemonger/spacetrader"
	"time"
)

type User struct {
	Id                string
	Username          string
	Token             string
	NewShipAssignment string
	NewShipSystem     string
	ShipMachines      []struct{}
	Loans             []spacetrader.Loan
	OutstandingLoans  int
	Credits           int
}

type Location struct {
	System       string
	SystemName   string
	Location     string
	LocationName string
	LocationType string
	X            int
	Y            int
	CreatedAt    time.Time
}
