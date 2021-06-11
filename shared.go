package spacemonger

import "time"

type Cargo struct {
	/// The good in the cargo
	Good string `json:"good"`
	/// The quantity of the good
	Quantity int `json:"quantity"`
	/// The total volume consumed by the good
	TotalVolume int `json:"totalVolume"`
}

type Ship struct {
	ID string `json:"id"`
	/// The current location of the ship or None if the ship is in transit
	Location string `json:"location"`
	/// Any cargo within the ship
	Cargo []Cargo `json:"cargo"`
	/// The volume available in the ships cargo
	SpaceAvailable int `json:"spaceAvailable"`
	/// The type of the ship
	ShipType string `json:"type"`
	/// The class of the ship
	Class string
	/// The maximum cargo volume of the ship
	MaxCargo int `json:"maxCargo"`
	/// The speed rating of the ship
	Speed int `json:"speed"`
	/// The manufacturer of the ship
	Manufacturer string `json:"manufacturer"`
	/// The defensive rating of the ship
	Plating int `json:"plating"`
	/// The offensive rating of the ship
	Weapons int `json:"weapons"`
	/// The ships current X coordinate
	X int `json:"x"`
	/// The ships current Y coordinate
	Y int `json:"y"`
	/// The ships current flight plan
	FlightPlanID string `json:"flightPlanId"`
}

type Loan struct {
	/// The id of the loan
	ID string
	/// The due date of the loan
	Due time.Time
	/// The repayment amount of the loan
	ReplaymentAmount int `json:"replaymentAmount"`
	/// The current loan status
	Status string
	/// The type of the loan
	LoanType string `json:"type"`
}
