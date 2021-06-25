package spacemonger

import (
	"context"
	"log"
	"time"

	"spacemonger/spacetraders"
)

type Ship struct {
	dbConn       DBConn
	user         User
	Id           string
	location     string
	ShipMessages chan ShipMessage
}

func NewShip(dbConn DBConn, u User, ship spacetraders.Ship) Ship {
	return Ship{
		dbConn:       dbConn,
		user:         u,
		Id:           ship.Id,
		location:     ship.Location,
		ShipMessages: u.ShipMessages,
	}
}

func (s Ship) Run(ctx context.Context) <-chan error {
	exit := make(chan error)
	go func() {
		for {
			log.Printf("%s -- Collecting marketplace for location %s\n", s.user.Username, s.location)
			marketplace, err := s.user.Client.GetLocationMarketplace(ctx, s.location)
			if err != nil {
				exit <- err
			}

			if err := SaveLocationMarketplaceRespones(ctx, s.dbConn, s.location, marketplace); err != nil {
				log.Printf("%s -- Unable to collect marketplace data\n", s.user.Username)
				exit <- err
			}

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
