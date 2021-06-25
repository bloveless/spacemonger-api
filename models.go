package spacemonger

import (
	"time"

	"spacemonger/spacetraders"
)

type DbUser struct {
	Id                string
	Username          string
	Token             string
	NewShipAssignment string
	NewShipSystem     string
	ShipMachines      []struct{}
	Loans             []spacetraders.Loan
	OutstandingLoans  int
	Credits           int
}

type DbLocation struct {
	System       string
	SystemName   string
	Location     string
	LocationName string
	LocationType string
	X            int
	Y            int
	CreatedAt    time.Time
}

type DbDistanceBetweenLocations struct {
	originLocationType string
	distance           float64
}

type DbFlightPlan struct {
	Id                     string
	UserId                 string
	ShipId                 string
	Origin                 string
	Destination            string
	Distance               float64
	FuelConsumed           int
	FuelRemaining          int
	TimeRemainingInSeconds int
	ArrivesAt              time.Time
	CreatedAt              time.Time
}

type DbRoute struct {
	PurchaseLocation          string
	PurchaseLocationType      string
	SellLocation              string
	Good                      string
	Distance                  float64
	PurchaseLocationQuantity  int
	SellLocationQuantity      int
	PurchasePricePerUnit      int
	SellPricePerUnit          int
	VolumePerUnit             int
	CostVolumeDistance        float64
	ProfitSpeedVolumeDistance float64
}
