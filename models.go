package spacemonger

import (
	"time"

	"spacemonger/spacetraders"
)

type DbUser struct {
	Id               string
	Username         string
	Token            string
	NewShipRoleData  RoleData
	ShipMachines     []struct{}
	Loans            []spacetraders.Loan
	OutstandingLoans int
	Credits          int
}

type RoleData struct {
	Role     string `json:"role"`
	System   string `json:"system"`
	Location string `json:"location"`
}

type DbShip struct {
	UserId       string
	ShipId       string
	Type         string
	Class        string
	MaxCargo     int
	LoadingSpeed int
	Speed        int
	Manufacturer string
	Plating      int
	Weapons      int
	RoleData     RoleData
	ModifiedAt   time.Time
	CreatedAt    time.Time
}

type DbLocation struct {
	System       string
	Location     string
	LocationName string
	Type         string
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
