package spacemonger

type ShipMessageType int

const (
	UpdateCredits ShipMessageType = iota
)

type ShipMessage struct {
	Type       ShipMessageType
	NewCredits int
}

func (mt ShipMessageType) String() string {
	return [...]string{"UpdateCredits"}[mt]
}

type ShipRole int

const (
	Trader ShipRole = iota
	Scout
)

func (sr ShipRole) String() string {
	return [...]string{"Trader", "Scout"}[sr]
}
