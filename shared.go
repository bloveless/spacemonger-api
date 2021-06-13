package spacemonger

import "time"

type Cargo struct {
	Good        string `json:"good"`
	Quantity    int    `json:"quantity"`
	TotalVolume int    `json:"totalVolume"`
}

type Ship struct {
	Id             string  `json:"id"`
	Location       string  `json:"location"`
	Cargo          []Cargo `json:"cargo"`
	SpaceAvailable int     `json:"spaceAvailable"`
	ShipType       string  `json:"type"`
	Class          string
	MaxCargo       int    `json:"maxCargo"`
	Speed          int    `json:"speed"`
	Manufacturer   string `json:"manufacturer"`
	Plating        int    `json:"plating"`
	Weapons        int    `json:"weapons"`
	X              int    `json:"x"`
	Y              int    `json:"y"`
	FlightPlanId   string `json:"flightPlanId"`
}

type Loan struct {
	Id              string    `json:"id"`
	Due             time.Time `json:"due"`
	RepaymentAmount int       `json:"repaymentAmount"`
	Status          string    `json:"status"`
	LoanType        string    `json:"type"`
}

type FlightPlanData struct {
	Id string `json:"id"`
	ShipId string `json:"shipId"`
	FuelConsumed int `json:"fuelConsumed"`
	FuelRemaining int `json:"fuelRemaining"`
	TimeRemainingInSeconds int `json:"timeRemainingInSeconds"`
	CreatedAt int `json:"createdAt"`
	ArrivesAt int `json:"arrivesAt"`
	TerminatedAt int `json:"terminatedAt"`
	Destination string `json:"destination"`
	Departure string `json:"departure"`
	Distance int `json:"distance"`
}
