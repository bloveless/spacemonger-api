package spacemonger

import (
	"context"
	"log"
	"time"

	"spacemonger/spacetraders"
)

type Cargo struct {
	Good        string
	Quantity    int
	TotalVolume int
}

type Ship struct {
	Username     string
	Id           string
	Location     string
	LoadingSpeed int
	MaxCargo     int
	Cargo        []Cargo
	RoleData     RoleData
}

func (s Ship) Run(ctx context.Context, conn DBConn, client spacetraders.AuthorizedClient) {
	for {
		log.Printf("%s:%s -- Collecting marketplace for location %s\n", s.Username, s.Id, s.Location)

		marketplace, err := client.GetLocationMarketplace(ctx, s.Location)
		if err != nil {
			// TODO: return or continue
			return
		}

		if err := SaveLocationMarketplaceResponses(ctx, conn, s.Location, marketplace); err != nil {
			log.Printf("%s:%s -- Unable to collect marketplace data\n", s.Username, s.Id)
			// TODO: return or continue
			return
		}

		// Phase 1: Fill up on fuel and fly to each location collecting marketplace data

		// locations, err := s.user.Client.GetSystemLocations(ctx, "OE")
		// if err != nil {
		// 	exit <- err
		// 	// TODO: return or continue
		// 	return
		// }

		log.Printf("%s:%s -- Saved marketplace data for location %s\n", s.Username, s.Id, s.Location)

		// s.ShipMessages <- ShipMessage{
		// 	Type:       UpdateCredits,
		// 	NewCredits: 100000,
		// }

		time.Sleep(60 * time.Second)
	}
}
