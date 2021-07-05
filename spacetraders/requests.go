package spacetraders

type CreateFlightPlanRequest struct {
	ShipId      string `json:"shipId"`
	Destination string `json:"destination"`
}

type CreateLoanRequest struct {
	Type string `json:"type"`
}

type CreatePurchaseOrderRequest struct {
	ShipId   string `json:"shipId"`
	Good     string `json:"good"`
	Quantity int    `json:"quantity"`
}

type CreateSellOrderRequest struct {
	ShipId   string `json:"shipId"`
	Good     string `json:"good"`
	Quantity int    `json:"quantity"`
}

type PurchaseShipRequest struct {
	Location string `json:"location"`
	Type     string `json:"type"`
}

type JettisonCargoRequest struct {
	Good     string `json:"good"`
	Quantity int    `json:"quantity"`
}

type WarpJumpRequest struct {
	ShipId string `json:"shipId"`
}
