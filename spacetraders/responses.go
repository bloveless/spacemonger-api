package spacetraders

import "time"

type GetMyIpAddressResponse struct {
	Ip string `json:"ip"`
}

type GameStatusResponse struct {
	Status string `json:"status"`
}

type ClaimUsernameResponseUser struct {
	Username string `json:"username"`
	Credits  int    `json:"credits"`
	Ships    []Ship `json:"ships"`
	Loans    []Loan `json:"loans"`
}

type ClaimUsernameResponse struct {
	Token string                    `json:"token"`
	User  ClaimUsernameResponseUser `json:"user"`
}

type UserInfoUser struct {
	Username       string    `json:"username"`
	Credits        int       `json:"credits"`
	ShipCount      int       `json:"shipCount"`
	StructureCount int       `json:"structureCount"`
	JoinedAt       time.Time `json:"joinedAt"`
}

type GetMyInfoResponse struct {
	User UserInfoUser `json:"user"`
}

type GetFlightPlanResponse struct {
	FlightPlan FlightPlan `json:"flightPlan"`
}

type CreateFlightPlanResponse struct {
	FlightPlan FlightPlan `json:"flightPlan"`
}

type GetMyLoansResponse struct {
	Loans []Loan `json:"loans"`
}

type PayOffLoanResponse struct {
	Credits int    `json:"credits"`
	Loans   []Loan `json:"loans"`
}

type CreateLoanResponse struct {
	Credits int  `json:"credits"`
	Loan    Loan `json:"loan"`
}

type GetLocationResponse struct {
	Location    LocationDetails `json:"location"`
	DockedShips int             `json:"dockedShips"`
}

type GetLocationMarketplaceResponse struct {
	Marketplace []MarketplaceData
}

type CreatePurchaseOrderResponse struct {
	Credits int       `json:"credits"`
	Order   OrderData `json:"order"`
	Ship    Ship      `json:"ship"`
}

type CreateSellOrderResponse struct {
	Credits int       `json:"credits"`
	Order   OrderData `json:"order"`
	Ship    Ship      `json:"ship"`
}

type PurchaseShipResponse struct {
	Credits int  `json:"credits"`
	Ship    Ship `json:"ship"`
}

type GetMyShipRequest struct {
	Ship Ship `json:"ship"`
}

type GetMyShipsResponse struct {
	Ships []Ship `json:"ships"`
}

type JettisonCargoResponse struct {
	ShipId            string `json:"shipId"`
	Good              string `json:"good"`
	QuantityRemaining int    `json:"quantityRemaining"`
}

type PurchaseLocation struct {
	System   string `json:"system"`
	Location string `json:"location"`
	Price    int    `json:"price"`
}

type ShipForSale struct {
	ShipType          string             `json:"type"`
	Class             string             `json:"class"`
	MaxCargo          int                `json:"maxCargo"`
	LoadingSpeed      int                `json:"loadingSpeed"`
	Speed             int                `json:"speed"`
	Manufacturer      string             `json:"manufacturer"`
	Plating           int                `json:"plating"`
	Weapons           int                `json:"weapons"`
	PurchaseLocations []PurchaseLocation `json:"purchaseLocations"`
	RestrictedGoods   []string           `json:"restrictedGoods"`
}

type GetShipsForSaleResponse struct {
	ShipsForSale []ShipForSale `json:"ships"`
}

type SystemLocation struct {
	Symbol             string   `json:"symbol"`
	SystemLocationType string   `json:"type"`
	Name               string   `json:"name"`
	X                  int      `json:"x"`
	Y                  int      `json:"y"`
	AllowsConstruction bool     `json:"allowsConstruction"`
	Traits             []string `json:"traits"`
	Messages           []string `json:"messages"`
}

type GetSystemLocationsResponse struct {
	Locations []SystemLocation `json:"locations"`
}

type AvailableLoan struct {
	LoanType           string  `json:"type"`
	Amount             int     `json:"amount"`
	Rate               float64 `json:"rate"`
	TermInDays         int     `json:"termInDays"`
	CollateralRequired bool    `json:"collateralRequired"`
}

type GetAvailableLoansResponse struct {
	Loans []AvailableLoan `json:"loans"`
}

type WarpJumpResponse struct {
	FlightPlan FlightPlan `json:"flightPlan"`
}
