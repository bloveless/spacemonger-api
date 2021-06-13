package spacemonger

import "time"

type MyIpAddressResponse struct {
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

type UserInfo struct {
	User UserInfoUser `json:"user"`
}

type FlightPlan struct {
	FlightPlan FlightPlanData `json:"flightPlan"`
}
