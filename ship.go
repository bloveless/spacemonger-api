package spacemonger

import (
	"context"
	"fmt"
	"log"
	"time"
)

type Ship struct {
	dbConn       DBConn
	user         User
	Id           string
	location     string
	ShipMessages chan ShipMessage
	Role         ShipRole
}

func ShipFromShipRow(dbConn DBConn, u User, ship ShipRow) (Ship, error) {
	fmt.Printf("ShipRow: %+v\n", ship)

	return Ship{
		dbConn:       dbConn,
		user:         u,
		Id:           ship.ShipId,
		location:     ship.Location,
		ShipMessages: u.ShipMessages,
		Role:         Scout,
	}, nil
}

func (s Ship) Run(ctx context.Context) <-chan error {
	exit := make(chan error)
	go func() {
		for {
			log.Printf("%s -- Collecting marketplace for location %s\n", s.user.Username, s.location)

			fmt.Printf("Ship: %+v\n", s)
			marketplace, err := s.user.Client.GetLocationMarketplace(ctx, s.location)
			if err != nil {
				exit <- err
				// TODO: return or continue
				return
			}

			if err := SaveLocationMarketplaceResponses(ctx, s.dbConn, s.location, marketplace); err != nil {
				log.Printf("%s -- Unable to collect marketplace data\n", s.user.Username)
				exit <- err
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

			log.Printf("%s -- Saved marketplace data for location %s\n", s.user.Username, s.location)

			// s.ShipMessages <- ShipMessage{
			// 	Type:       UpdateCredits,
			// 	NewCredits: 100000,
			// }

			time.Sleep(60 * time.Second)
		}
	}()

	return exit
}
