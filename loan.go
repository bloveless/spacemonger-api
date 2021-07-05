package spacemonger

import "time"

type Loan struct {
	Id              string    `json:"id"`
	Due             time.Time `json:"due"`
	RepaymentAmount int       `json:"repaymentAmount"`
	Status          string    `json:"status"`
	Type            string    `json:"type"`
}
