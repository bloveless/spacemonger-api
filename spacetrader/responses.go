package spacetrader

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

type GetShipsForSaleResponse struct {
	Ships []Ship `json:"ships"`
}

type GetSystemsInfoResponse struct {
	Systems []System `json:"systems"`
}

type AvailableLoan struct {
	LoanType           string  `json:"type"`
	Amount             int     `json:"amount"`
	Rate               float64 `json:"rate"`
	TermInDays         int     `json:"termInDays"`
	CollateralRequired bool    `json:"collateralRequired"`
}

type GetAvailableLoansResponse struct {
	Loans []AvailableLoan `json:"loans'"`
}

type WarpJumpResponse struct {
	FlightPlan FlightPlan `json:"flightPlan"`
}
