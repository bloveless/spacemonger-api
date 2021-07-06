package spacemonger

import (
	"context"
	"errors"
	"fmt"
	"log"
	"time"

	"spacemonger/spacetraders"

	"github.com/jackc/pgx/v4"
)

type Cargo struct {
	Good        string
	Quantity    int
	TotalVolume int
}

type Ship struct {
	Username       string
	UserId         string
	Id             string
	Type           string
	Location       string
	LoadingSpeed   int
	Speed          int
	MaxCargo       int
	SpaceAvailable int
	Cargo          []Cargo
	RoleData       RoleData
	Messages       chan ShipMessage
}

func (s *Ship) emptyCargo(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient) error {
	// Empty cargo
	for _, c := range s.Cargo {
		err := s.sellGood(ctx, conn, client, c.Good, c.Quantity)
		if err != nil {
			return fmt.Errorf("unable to create sell order while emptying cargo: %w", err)
		}
	}

	return nil
}

func (s *Ship) purchaseGood(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient, good string, quantity int) error {
	goodRemainingToPurchase := quantity
	for goodRemainingToPurchase > 0 {
		purchaseQuantity := goodRemainingToPurchase
		if s.LoadingSpeed < goodRemainingToPurchase {
			purchaseQuantity = s.LoadingSpeed
		}

		newCredits, err := CreatePurchaseOrder(ctx, client, conn, s, good, purchaseQuantity)
		if err != nil {
			return fmt.Errorf("unable to create purchase order while purchasing good: %w", err)
		}

		log.Printf("%s:%s -- Purchased good \"%s\" quantity \"%d\"\n", s.Username, s.Id, good, purchaseQuantity)
		s.Messages <- ShipMessage{
			Type:       ShipMessageUpdateCredits,
			ShipId:     s.Id,
			NewCredits: newCredits,
		}

		goodRemainingToPurchase -= purchaseQuantity
	}

	return nil
}

func (s *Ship) sellGood(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient, good string, quantity int) error {
	goodRemainingToSell := quantity
	for goodRemainingToSell > 0 {
		sellQuantity := goodRemainingToSell
		if s.LoadingSpeed < goodRemainingToSell {
			sellQuantity = s.LoadingSpeed
		}

		newCredits, err := CreateSellOrder(ctx, client, conn, s, good, sellQuantity)
		if err != nil {
			return fmt.Errorf("unable to create purchase order while selling good: %w", err)
		}

		log.Printf("%s:%s -- Sold good \"%s\" quantity \"%d\"\n", s.Username, s.Id, good, quantity)
		s.Messages <- ShipMessage{
			Type:       ShipMessageUpdateCredits,
			ShipId:     s.Id,
			NewCredits: newCredits,
		}

		goodRemainingToSell -= sellQuantity
	}

	return nil
}

func (s *Ship) purchaseFuelForTrip(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient, destination string) error {
	fuelRequired, err := GetAdditionalFuelRequiredForTrip(ctx, client, conn, *s, destination)
	if err != nil {
		return fmt.Errorf("unable to get required fuel: %w", err)
	}

	log.Printf("%s:%s -- Fuel Required to travel from %s to %s for ship type %s is %d\n", s.Username, s.Id, s.Location, destination, s.Type, fuelRequired)

	err = s.purchaseGood(ctx, conn, client, GoodFuel, fuelRequired)
	if err != nil {
		return fmt.Errorf("error purchasing fuel for trip: %w", err)
	}

	return nil
}

func (s *Ship) moveToLocation(ctx context.Context, conn DbConn, client spacetraders.AuthorizedClient, destination string) error {
	flightPlan, err := CreateFlightPlan(ctx, client, conn, s, destination)
	if err != nil {
		return fmt.Errorf("unable to create flight plan: %w", err)
	}

	log.Printf("%s:%s -- Flight plan created. Waiting for %d seconds for ship to arrive\n", s.Username, s.Id, flightPlan.TimeRemainingInSeconds)
	time.Sleep(time.Duration(flightPlan.TimeRemainingInSeconds) * time.Second)

	s.Location = destination
	err = UpdateShipLocation(ctx, conn, *s, s.Location)
	if err != nil {
		return fmt.Errorf("unable to update ships location in db: %w", err)
	}

	return nil
}

func (s Ship) Run(ctx context.Context, config Config, conn DbConn, client spacetraders.AuthorizedClient) {
	// Pre-flight checks.
	// 1. Make sure that the ship isn't currently in motion. If it is we need to wait for it to arrive.
	// 2. Make sure that the ship starts with empty cargo
	flightPlan, err := GetActiveFlightPlan(ctx, conn, s.Id)
	if err != nil && !errors.Is(err, pgx.ErrNoRows) {
		log.Printf("%s:%s -- ERROR looking up flight plan: %v", s.Username, s.Id, err)
	}

	if err == nil {
		log.Printf("%s:%s -- Ship is currently in motion to \"%s\". Sleeping until it arrives at %v\n", s.Username, s.Id, flightPlan.Destination, flightPlan.ArrivesAt)
		time.Sleep(time.Until(flightPlan.ArrivesAt))
		s.Location = flightPlan.Destination
	}

	err = s.emptyCargo(ctx, conn, client)
	if err != nil {
		log.Printf("%s:%s -- Unable to empty cargo during pre-flight check... continuing anyway: %s", s.Username, s.Id, err)
	}

	for {
		if s.RoleData.Role == "Trader" {
			if err := s.emptyCargo(ctx, conn, client); err != nil {
				log.Printf("%s:%s -- ERROR unable to empty cargo: %s\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			tradeRoute, err := GetBestTradingRoute(ctx, conn, s)
			if err != nil {
				log.Printf("%s:%s -- Unable to find a trade route from \"%s\"\n", s.Username, s.Id, s.Location)
				time.Sleep(60 * time.Second)
				continue
			}

			if config.EnableTraderLogs {
				log.Printf("%s:%s -- Found a trade route %+v\n", s.Username, s.Id, tradeRoute)
			}

			if err := s.purchaseFuelForTrip(ctx, conn, client, tradeRoute.SellLocation); err != nil {
				log.Printf("%s:%s -- ERROR unable to purchase fuel for trip to trade route sell location: %s\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			maxQuantityToBuy := s.SpaceAvailable / tradeRoute.VolumePerUnit
			if err := s.purchaseGood(ctx, conn, client, tradeRoute.Good, maxQuantityToBuy); err != nil {
				log.Printf("%s:%s -- ERROR unable to purchase good \"%s\" quantity: \"%d\" to trade: %s\n", s.Username, s.Id, tradeRoute.Good, maxQuantityToBuy, err)
				time.Sleep(60 * time.Second)
				continue
			}

			if err := s.moveToLocation(ctx, conn, client, tradeRoute.SellLocation); err != nil {
				log.Printf("%s:%s -- ERROR unable to move to sell location \"%s\": %s\n", s.Username, s.Id, tradeRoute.SellLocation, err)
				time.Sleep(60 * time.Second)
				continue
			}

			// Now that we've purchased the fuel and the good and moved to the trade location we can loop
			// which will sell the good pick a new trade location and start the process over again
		}

		if s.RoleData.Role == "Scout" {
			if s.Location != s.RoleData.Location {
				err := s.emptyCargo(ctx, conn, client)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to empty cargo: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
					continue
				}

				err = s.purchaseFuelForTrip(ctx, conn, client, s.RoleData.Location)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to purchase fuel for trip to trade route sell location: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
				}

				err = s.moveToLocation(ctx, conn, client, s.RoleData.Location)
				if err != nil {
					log.Printf("%s:%s -- ERROR trying to move ship to location: %v\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
					continue
				}
			}

			if config.EnableScoutLogs {
				log.Printf("%s:%s -- Scout is currently assigned to location %s in system %s\n", s.Username, s.Id, s.RoleData.Location, s.RoleData.System)
				log.Printf("%s:%s -- Collecting marketplace for location %s\n", s.Username, s.Id, s.Location)
			}

			marketplace, err := client.GetLocationMarketplace(ctx, s.Location)
			if err != nil {
				log.Printf("%s:%s -- Unable to get location marketplace data: %v\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			if err := SaveLocationMarketplaceResponses(ctx, conn, s.Location, marketplace); err != nil {
				log.Printf("%s:%s -- Unable to save marketplace data: %v\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			if config.EnableScoutLogs {
				log.Printf("%s:%s -- Saved marketplace data for location %s\n", s.Username, s.Id, s.Location)
			}

			s.Messages <- ShipMessage{
				Type:   ShipMessageNoop,
				ShipId: s.Id,
			}

			time.Sleep(60 * time.Second)
		}
	}
}
