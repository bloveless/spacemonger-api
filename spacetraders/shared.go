package spacetraders

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
	Type           string  `json:"type"`
	Class          string  `json:"class"`
	MaxCargo       int     `json:"maxCargo"`
	LoadingSpeed   int     `json:"loadingSpeed"`
	Speed          int     `json:"speed"`
	Manufacturer   string  `json:"manufacturer"`
	Plating        int     `json:"plating"`
	Weapons        int     `json:"weapons"`
	X              int     `json:"x"`
	Y              int     `json:"y"`
	FlightPlanId   string  `json:"flightPlanId"`
}

type Loan struct {
	Id              string    `json:"id"`
	Due             time.Time `json:"due"`
	RepaymentAmount int       `json:"repaymentAmount"`
	Status          string    `json:"status"`
	Type            string    `json:"type"`
}

type FlightPlan struct {
	Id                     string    `json:"id"`
	ShipId                 string    `json:"shipId"`
	FuelConsumed           int       `json:"fuelConsumed"`
	FuelRemaining          int       `json:"fuelRemaining"`
	TimeRemainingInSeconds int       `json:"timeRemainingInSeconds"`
	CreatedAt              time.Time `json:"createdAt"`
	ArrivesAt              time.Time `json:"arrivesAt"`
	TerminatedAt           time.Time `json:"terminatedAt"`
	Destination            string    `json:"destination"`
	Departure              string    `json:"departure"`
	Distance               int       `json:"distance"`
}

type Structure struct {
	Id       string `json:"id"`
	Type     string `json:"structureType"`
	Location string `json:"location"`
}

type LocationDetails struct {
	Symbol             string      `json:"symbol"`
	Type               string      `json:"type"`
	Name               string      `json:"name"`
	X                  int         `json:"x"`
	Y                  int         `json:"y"`
	AnsibleProgress    float64     `json:"ansibleProgress"`
	Anomaly            string      `json:"anomaly"`
	Structures         []Structure `json:"structures"`
	Messages           []string    `json:"messages"`
	AllowsConstruction bool        `json:"allowsConstruction"`
}

type MarketplaceData struct {
	Good                 string `json:"symbol"`
	VolumePerUnit        int    `json:"volumePerUnit"`
	PurchasePricePerUnit int    `json:"purchasePricePerUnit"`
	SellPricePerUnit     int    `json:"sellPricePerUnit"`
	QuantityAvailable    int    `json:"quantityAvailable"`
}

type OrderData struct {
	Good         string `json:"good"`
	Quantity     int    `json:"quantity"`
	PricePerUnit int    `json:"pricePerUnit"`
	Total        int    `json:"total"`
}
