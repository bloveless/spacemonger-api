package spacemonger

type MyIpAddressResponse struct {
	Ip string `json:"ip"`
}

type GameStatusResponse struct {
	Status string `json:"status"`
}

type ClaimUsernameUser struct {
	Username string `json:"username"`
	Credits  int    `json:"credits"`
	Ships    []Ship `json:"ships"`
	Loans    []Loan `json:"loans"`
}

type ClaimUsernameResponse struct {
	Token string            `json:"token"`
	User  ClaimUsernameUser `json:"user"`
}
