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

type DistanceBetweenLocations struct {
	originLocationType string
	distance           float64
}

type FlightPlan struct {
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

type Route struct {
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
	FuelRequired              float64
	FlightTime                float64
	CostVolumeDistance        float64
	ProfitSpeedVolumeDistance float64
}
