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
	Messages     chan ShipMessage
}

func (s Ship) Run(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient) {
	for {
		if s.RoleData.Role == "Trader" {
			log.Println("%s:%s -- Traders don't do anything right now\n", s.Username, s.Id)
			time.Sleep(60 * time.Second)
		}

		if s.RoleData.Role == "Scout" {
			log.Printf("%s:%s -- Scout is currently assigned to location %s in system %s\n", s.Username, s.Id, s.RoleData.Location, s.RoleData.System)

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

			// s.Messages <- ShipMessage{
			// 	Type:       UpdateCredits,
			// 	NewCredits: 100000,
			// }

			time.Sleep(60 * time.Second)
		}
	}
}
