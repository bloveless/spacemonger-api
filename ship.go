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
			log.Printf("%s:%s -- Traders don't do anything right now\n", s.Username, s.Id)

			s.Messages <- ShipMessage{
				Type: Noop,
			}

			time.Sleep(60 * time.Second)
		}

		if s.RoleData.Role == "Scout" {

			if s.Location != s.RoleData.Location {
				// Empty cargo
				for _, c := range s.Cargo {
					resp, err := client.CreateSellOrder(ctx, s.Id, c.Good, c.Quantity)
					if err != nil {
						// TODO: How should we handle errors? Send a message and restart the loop?
						//       For now I guess I'll just print it and continue
						log.Printf("%s:%s -- ERROR During create sell order: %v", s.Username, s.Id, err)
						continue
					}

					s.Messages <- ShipMessage{
						Type:       UpdateCredits,
						NewCredits: resp.Credits,
					}
				}

				// Purchase fuel
				// Move to location
				// Wait for arrival
				// Begin harvesting
			}

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

			s.Messages <- ShipMessage{
				Type: Noop,
			}

			time.Sleep(60 * time.Second)
		}
	}
}
