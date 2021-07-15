package spacemonger

import (
	"time"
)

type DbShipStats struct {
	Id           string   `json:"id"`
	Type         string   `json:"type"`
	Location     string   `json:"location"`
	LoadingSpeed int      `json:"loading_speed"`
	MaxCargo     int      `json:"max_cargo"`
	Cargo        []Cargo  `json:"cargo"`
	RoleData     RoleData `json:"role_data"`
}

type DbUserLatestStats struct {
	Id              string        `json:"id"`
	Username        string        `json:"username"`
	NewShipRoleData RoleData      `json:"new_ship_role_data"`
	Credits         int           `json:"credits"`
	ShipCount       int           `json:"ship_count"`
	Ships           []DbShipStats `json:"ships"`
	StatsUpdatedAt  time.Time     `json:"stats_updated_at"`
}

type DbUserStats struct {
	Id        int       `json:"id"`
	Credits   int       `json:"credits"`
	ShipCount int       `json:"ship_count"`
	CreatedAt time.Time `json:"created_at"`
}

type RoleData struct {
	Role     string `json:"role"`
	System   string `json:"system"`
	Location string `json:"location"`
}

type DbShip struct {
	UserId       string    `json:"user_id"`
	ShipId       string    `json:"ship_id"`
	Type         string    `json:"type"`
	Class        string    `json:"class"`
	MaxCargo     int       `json:"max_cargo"`
	LoadingSpeed int       `json:"loading_speed"`
	Speed        int       `json:"speed"`
	Manufacturer string    `json:"manufacturer"`
	Plating      int       `json:"plating"`
	Weapons      int       `json:"weapons"`
	RoleData     RoleData  `json:"role_data"`
	Location     string    `json:"location"`
	ModifiedAt   time.Time `json:"modified_at"`
	CreatedAt    time.Time `json:"created_at"`
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
	Distance               int
	FuelConsumed           int
	FuelRemaining          int
	TimeRemainingInSeconds int
	ArrivesAt              time.Time
	CreatedAt              time.Time
}

type DbRoute struct {
	PurchaseLocation         string
	PurchaseLocationType     string
	SellLocation             string
	Good                     string
	Distance                 float64
	PurchaseLocationQuantity int
	SellLocationQuantity     int
	PurchasePricePerUnit     int
	SellPricePerUnit         int
	VolumePerUnit            int
}

type DbTransaction struct {
	UserId       string    `json:"user_id"`
	ShipId       string    `json:"ship_id"`
	Type         string    `json:"type"`
	Good         string    `json:"good"`
	PricePerUnit int       `json:"price_per_unit"`
	Quantity     int       `json:"quantity"`
	Total        int       `json:"total"`
	Location     string    `json:"location"`
	CreatedAt    time.Time `json:"created_at"`
}
