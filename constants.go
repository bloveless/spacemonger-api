package spacemonger

type ShipMessageType int

const (
	UpdateCredits ShipMessageType = iota
	Noop
)

type ShipMessage struct {
	Type       ShipMessageType
	NewCredits int
}

func (mt ShipMessageType) String() string {
	return [...]string{"UpdateCredits", "Noop"}[mt]
}

type ShipRole int

const (
	Trader ShipRole = iota
	Scout
)

func (sr ShipRole) String() string {
	return [...]string{"Trader", "Scout"}[sr]
}

const (
	GoodFuel                  = "FUEL"
	GoodChemicals             = "CHEMICALS"
	GoodMetals                = "METALS"
	GoodDrones                = "DRONES"
	GoodFood                  = "FOOD"
	GoodConsumerGoods         = "CONSUMER_GOODS"
	GoodExplosives            = "EXPLOSIVES"
	GoodNarcotics             = "NARCOTICS"
	GoodTextiles              = "TEXTILES"
	GoodElectronics           = "ELECTRONICS"
	GoodMachinery             = "MACHINERY"
	GoodConstructionMaterials = "CONSTRUCTION_MATERIALS"
	GoodShipPlating           = "SHIP_PLATING"
	GoodRareMetals            = "RARE_METALS"
	GoodProteinSynthesizers   = "PROTEIN_SYNTHESIZERS"
	GoodResearch              = "RESEARCH"
	GoodPrecisionInstruments  = "PRECISION_INSTRUMENTS"
	GoodNanobots              = "NANOBOTS"
	GoodBiometricFirearms     = "BIOMETRIC_FIREARMS"
	GoodShipParts             = "SHIP_PARTS"
	GoodExoticPlasma          = "EXOTIC_PLASMA"
	GoodFusionReactors        = "FUSION_REACTORS"
	GoodZucoCrystals          = "ZUCO_CRYSTALS"
	GoodUnstableCompounds     = "UNSTABLE_COMPOUNDS"
)
