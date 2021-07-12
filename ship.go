package spacemonger

import (
	"context"
	"errors"
	"fmt"
	"log"
	"regexp"
	"strconv"
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
	Username              string
	UserId                string
	Id                    string
	Type                  string
	Location              string
	LoadingSpeed          int
	Speed                 int
	MaxCargo              int
	SpaceAvailable        int
	Cargo                 []Cargo
	RoleData              RoleData
	Messages              chan ShipMessage
	shipRepository        ShipRepository
	flightPlanRepository  FlightPlanRepository
	routeRepository       RouteRepository
	marketplaceRepository MarketplaceRepository
	transactionRepository TransactionRepository
}

type Route struct {
	PurchaseLocation          string
	PurchaseLocationType      string
	SellLocation              string
	Good                      string
	Distance                  float64
	PurchaseLocationQuantity  int
	SellLocationQuantity      int
	PurchasePricePerUnit      int
	SellPricePerUnit          int
	VolumePerUnit             int
	CostVolumeDistance        float64
	ProfitSpeedVolumeDistance float64
}

func (s *Ship) emptyCargo(ctx context.Context, client spacetraders.AuthorizedClient) error {
	// Empty cargo
	for _, c := range s.Cargo {
		err := s.sellGood(ctx, client, c.Good, c.Quantity)
		if err != nil {
			return fmt.Errorf("unable to create sell order while emptying cargo: %w", err)
		}
	}

	return nil
}

func (s *Ship) purchaseGood(ctx context.Context, client spacetraders.AuthorizedClient, good string, quantity int) error {
	goodRemainingToPurchase := quantity
	for goodRemainingToPurchase > 0 {
		purchaseQuantity := goodRemainingToPurchase
		if s.LoadingSpeed < goodRemainingToPurchase {
			purchaseQuantity = s.LoadingSpeed
		}

		newCredits, err := s.createPurchaseOrder(ctx, client, good, purchaseQuantity)
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

func (s *Ship) createPurchaseOrder(ctx context.Context, client spacetraders.AuthorizedClient, good string, quantity int) (int, error) {
	if quantity > 0 {
		resp, err := client.CreatePurchaseOrder(ctx, s.Id, good, quantity)
		if err != nil {
			return 0, fmt.Errorf("unable to create purchase order for good \"%s\" quantity \"%d\": %w", good, quantity, err)
		}

		var newCargo []Cargo
		for _, c := range resp.Ship.Cargo {
			newCargo = append(newCargo, Cargo(c))
		}

		s.Cargo = newCargo
		s.SpaceAvailable = resp.Ship.SpaceAvailable

		err = s.transactionRepository.SaveTransaction(ctx, DbTransaction{
			UserId:       s.UserId,
			ShipId:       s.Id,
			Type:         "purchase",
			Good:         good,
			PricePerUnit: resp.Order.PricePerUnit,
			Quantity:     resp.Order.Quantity,
			Total:        resp.Order.Total,
			Location:     resp.Ship.Location,
		})

		if err != nil {
			return 0, fmt.Errorf("unable to save purchase order to db: %w", err)
		}

		return resp.Credits, nil
	} else {
		return 0, fmt.Errorf("refusing to attempt to create purchase order with 0 quantity")
	}
}

func (s *Ship) sellGood(ctx context.Context, client spacetraders.AuthorizedClient, good string, quantity int) error {
	goodRemainingToSell := quantity
	for goodRemainingToSell > 0 {
		sellQuantity := goodRemainingToSell
		if s.LoadingSpeed < goodRemainingToSell {
			sellQuantity = s.LoadingSpeed
		}

		newCredits, err := s.createSellOrder(ctx, client, good, sellQuantity)
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

func (s *Ship) createSellOrder(ctx context.Context, client spacetraders.AuthorizedClient, good string, quantity int) (int, error) {
	if quantity > 0 {
		resp, err := client.CreateSellOrder(ctx, s.Id, good, quantity)
		if err != nil {
			return 0, fmt.Errorf("unable to create sell order for good \"%s\" quantity \"%d\": %w", good, quantity, err)
		}

		var newCargo []Cargo
		for _, c := range resp.Ship.Cargo {
			newCargo = append(newCargo, Cargo(c))
		}

		s.Cargo = newCargo
		s.SpaceAvailable = resp.Ship.SpaceAvailable

		err = s.transactionRepository.SaveTransaction(ctx, DbTransaction{
			UserId:       s.UserId,
			ShipId:       s.Id,
			Type:         "sell",
			Good:         good,
			PricePerUnit: resp.Order.PricePerUnit,
			Quantity:     resp.Order.Quantity,
			Total:        resp.Order.Total,
			Location:     resp.Ship.Location,
		})

		if err != nil {
			return 0, fmt.Errorf("unable to save transaction to db: %w", err)
		}

		return resp.Credits, nil
	} else {
		return 0, fmt.Errorf("refusing to attempt to create sell order with 0 quantity")
	}
}

func (s *Ship) purchaseFuelForTrip(ctx context.Context, client spacetraders.AuthorizedClient, destination string) error {
	fuelRequired, err := s.getAdditionalFuelRequiredForTrip(ctx, client, destination)
	if err != nil {
		return fmt.Errorf("unable to get required fuel: %w", err)
	}

	log.Printf("%s:%s -- Fuel Required to travel from %s to %s for ship type %s is %d\n", s.Username, s.Id, s.Location, destination, s.Type, fuelRequired)

	err = s.purchaseGood(ctx, client, GoodFuel, fuelRequired)
	if err != nil {
		return fmt.Errorf("error purchasing fuel for trip: %w", err)
	}

	return nil
}

func (s *Ship) getAdditionalFuelRequiredForTrip(ctx context.Context, client spacetraders.AuthorizedClient, destination string) (int, error) {
	currentFuel := 0
	for _, c := range s.Cargo {
		if c.Good == GoodFuel {
			currentFuel += c.Quantity
		}
	}

	log.Printf("%s:%s -- Ship currently has %d fuel\n", s.Username, s.Id, currentFuel)

	dbFuelRequired, err := s.flightPlanRepository.GetFuelRequired(ctx, s.Location, destination, s.Type)
	if err == nil {
		log.Printf("%s:%s -- Using fuel required from the db %d\n", s.Username, s.Id, dbFuelRequired)
		if dbFuelRequired > currentFuel {
			return dbFuelRequired - currentFuel, nil
		}

		return 0, nil
	}

	// At this point we weren't able to look up the fuel required so we should prepare our ship by selling all the fuel,
	// attempting to make a flight plan and return how much fuel is required to make the flight

	log.Printf("%s:%s -- Selling %d fuel in order to get fuel required from Api\n", s.Username, s.Id, currentFuel)

	if currentFuel > 0 {
		sellOrder, err := client.CreateSellOrder(ctx, s.Id, GoodFuel, currentFuel)
		if err != nil {
			return 0, err
		}

		s.Messages <- ShipMessage{
			Type:       ShipMessageUpdateCredits,
			ShipId:     s.Id,
			NewCredits: sellOrder.Credits,
		}
	}

	_, err = client.CreateFlightPlan(ctx, s.Id, destination)
	if err == nil {
		return 0, fmt.Errorf("create flight plan should have failed... ship is now in motion")
	}

	log.Printf("%s:%s -- Received error message from CreateFlightPlan (this is expected) %s\n", s.Username, s.Id, err.Error())

	re := regexp.MustCompile(`You require (\d+?) more FUEL`) // want to know what is in front of 'at'
	requiredFuelMatches := re.FindStringSubmatch(err.Error())

	log.Printf("%s:%s -- %s additional fuel is required", s.Username, s.Id, requiredFuelMatches[1])

	requiredFuel, err := strconv.Atoi(requiredFuelMatches[1])
	if err != nil {
		return 0, err
	}

	return requiredFuel, nil
}

func (s *Ship) createFlightPlan(ctx context.Context, client spacetraders.AuthorizedClient, destination string) (FlightPlan, error) {
	flightPlanResp, err := client.CreateFlightPlan(ctx, s.Id, destination)
	if err != nil {
		return FlightPlan{}, err
	}

	s.Location = ""
	var newCargo []Cargo
	for _, c := range s.Cargo {
		if c.Good == GoodFuel {
			c.Quantity -= flightPlanResp.FlightPlan.FuelConsumed
		}

		newCargo = append(newCargo, c)
	}

	s.Cargo = newCargo

	err = s.flightPlanRepository.SaveFlightPlan(ctx, s.UserId, DbFlightPlan{
		Id:                     flightPlanResp.FlightPlan.Id,
		UserId:                 s.UserId,
		ShipId:                 s.Id,
		Origin:                 flightPlanResp.FlightPlan.Departure,
		Destination:            flightPlanResp.FlightPlan.Destination,
		Distance:               flightPlanResp.FlightPlan.Distance,
		FuelConsumed:           flightPlanResp.FlightPlan.FuelConsumed,
		FuelRemaining:          flightPlanResp.FlightPlan.FuelRemaining,
		TimeRemainingInSeconds: flightPlanResp.FlightPlan.TimeRemainingInSeconds,
		ArrivesAt:              flightPlanResp.FlightPlan.ArrivesAt,
		CreatedAt:              flightPlanResp.FlightPlan.CreatedAt,
	})
	if err != nil {
		return FlightPlan{}, err
	}

	return FlightPlan{
		Id:                     flightPlanResp.FlightPlan.Id,
		ShipId:                 s.Id,
		FuelConsumed:           flightPlanResp.FlightPlan.FuelConsumed,
		FuelRemaining:          flightPlanResp.FlightPlan.FuelRemaining,
		TimeRemainingInSeconds: flightPlanResp.FlightPlan.TimeRemainingInSeconds,
		CreatedAt:              flightPlanResp.FlightPlan.CreatedAt,
		ArrivesAt:              flightPlanResp.FlightPlan.ArrivesAt,
		TerminatedAt:           flightPlanResp.FlightPlan.TerminatedAt,
		Origin:                 flightPlanResp.FlightPlan.Departure,
		Destination:            flightPlanResp.FlightPlan.Destination,
		Distance:               flightPlanResp.FlightPlan.Distance,
	}, nil
}

func (s *Ship) moveToLocation(ctx context.Context, client spacetraders.AuthorizedClient, destination string) error {
	flightPlan, err := s.createFlightPlan(ctx, client, destination)
	if err != nil {
		return fmt.Errorf("unable to create flight plan: %w", err)
	}

	log.Printf("%s:%s -- Flight plan created. Waiting for %d seconds for ship to arrive\n", s.Username, s.Id, flightPlan.TimeRemainingInSeconds)
	time.Sleep(time.Duration(flightPlan.TimeRemainingInSeconds) * time.Second)

	s.Location = destination
	err = s.shipRepository.UpdateShipLocation(ctx, *s, s.Location)
	if err != nil {
		return fmt.Errorf("unable to update ships location in db: %w", err)
	}

	return nil
}

func (s *Ship) getBestTradingRoute(ctx context.Context) (Route, error) {
	log.Printf("Getting routes for ship from location \"%s\"\n", s.Location)
	dbRoutes, err := s.routeRepository.GetRoutes(ctx, s.Location)
	if err != nil {
		return Route{}, fmt.Errorf("unable to get routes from db: %w", err)
	}

	foundARoute := false
	bestRoute := Route{}
	for _, r := range dbRoutes {
		profit := float64(r.SellPricePerUnit - r.PurchasePricePerUnit)
		costVolumeDistance := profit / float64(r.VolumePerUnit) / r.Distance
		profitSpeedVolumeDistance := (profit * float64(s.Speed)) / (float64(r.VolumePerUnit) * r.Distance)

		route := Route{
			PurchaseLocation:          r.PurchaseLocation,
			PurchaseLocationType:      r.PurchaseLocationType,
			SellLocation:              r.SellLocation,
			Good:                      r.Good,
			Distance:                  r.Distance,
			PurchaseLocationQuantity:  r.PurchaseLocationQuantity,
			SellLocationQuantity:      r.SellLocationQuantity,
			PurchasePricePerUnit:      r.PurchasePricePerUnit,
			SellPricePerUnit:          r.SellPricePerUnit,
			VolumePerUnit:             r.VolumePerUnit,
			CostVolumeDistance:        costVolumeDistance,
			ProfitSpeedVolumeDistance: profitSpeedVolumeDistance,
		}

		if route.SellLocation == "OE-W-XV" {
			continue
		}

		if route.PurchaseLocationQuantity < 500 {
			continue
		}

		// We must allow trades that will cost us money so we don't get stuck at any location
		// if route.ProfitSpeedVolumeDistance <= 0.0 {
		// 	continue
		// }

		if !foundARoute || (route.ProfitSpeedVolumeDistance > bestRoute.ProfitSpeedVolumeDistance) {
			foundARoute = true
			bestRoute = route
		}
	}

	if !foundARoute {
		return Route{}, fmt.Errorf("unable to find any trade route from \"%s\"", s.Location)
	}

	return bestRoute, nil
}

func (s Ship) Run(ctx context.Context, config Config, conn DbConn, client spacetraders.AuthorizedClient) {
	// Pre-flight checks.
	// 1. Make sure that the ship isn't currently in motion. If it is we need to wait for it to arrive.
	// 2. Make sure that the ship starts with empty cargo
	flightPlan, err := s.flightPlanRepository.GetActiveFlightPlan(ctx, s.Id)
	if err != nil && !errors.Is(err, pgx.ErrNoRows) {
		log.Printf("%s:%s -- ERROR looking up flight plan: %v", s.Username, s.Id, err)
	}

	if err == nil {
		log.Printf("%s:%s -- Ship is currently in motion to \"%s\". Sleeping until it arrives at %v\n", s.Username, s.Id, flightPlan.Destination, flightPlan.ArrivesAt)
		time.Sleep(time.Until(flightPlan.ArrivesAt))
		s.Location = flightPlan.Destination
	}

	err = s.emptyCargo(ctx, client)
	if err != nil {
		log.Printf("%s:%s -- Unable to empty cargo during pre-flight check... continuing anyway: %s", s.Username, s.Id, err)
	}

	for {
		if s.RoleData.Role == "Trader" {
			if s.Location == "" {
				log.Printf("%s:%s -- SOME STRANGE the ship has forgotten it's location", s.Username, s.Id)

				newShip, err := client.GetMyShip(ctx, s.Id)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to reload ship while trying to refetch it's location: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
					continue
				}

				log.Printf("%s:%s -- Updating ships location to \"%s\"\n", s.Username, s.Id, newShip.Ship.Location)
				s.Location = newShip.Ship.Location
				err = s.shipRepository.UpdateShipLocation(ctx, s, s.Location)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to update ships location in db: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
					continue
				}
			}

			if err := s.emptyCargo(ctx, client); err != nil {
				log.Printf("%s:%s -- ERROR unable to empty cargo: %s\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			tradeRoute, err := s.getBestTradingRoute(ctx)
			if err != nil {
				log.Printf("%s:%s -- Unable to find a trade route from \"%s\"\n", s.Username, s.Id, s.Location)
				time.Sleep(60 * time.Second)
				continue
			}

			if config.EnableTraderLogs {
				log.Printf("%s:%s -- Found a trade route %+v\n", s.Username, s.Id, tradeRoute)
			}

			if err := s.purchaseFuelForTrip(ctx, client, tradeRoute.SellLocation); err != nil {
				log.Printf("%s:%s -- ERROR unable to purchase fuel for trip to trade route sell location: %s\n", s.Username, s.Id, err)
				time.Sleep(60 * time.Second)
				continue
			}

			maxQuantityToBuy := s.SpaceAvailable / tradeRoute.VolumePerUnit
			if err := s.purchaseGood(ctx, client, tradeRoute.Good, maxQuantityToBuy); err != nil {
				log.Printf("%s:%s -- ERROR unable to purchase good \"%s\" quantity: \"%d\" to trade: %s\n", s.Username, s.Id, tradeRoute.Good, maxQuantityToBuy, err)
				time.Sleep(60 * time.Second)
				continue
			}

			if err := s.moveToLocation(ctx, client, tradeRoute.SellLocation); err != nil {
				log.Printf("%s:%s -- ERROR unable to move to sell location \"%s\": %s\n", s.Username, s.Id, tradeRoute.SellLocation, err)
				time.Sleep(60 * time.Second)
				continue
			}

			// Now that we've purchased the fuel and the good and moved to the trade location we can loop
			// which will sell the good pick a new trade location and start the process over again
		}

		if s.RoleData.Role == "Scout" {
			if s.Location != s.RoleData.Location {
				err := s.emptyCargo(ctx, client)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to empty cargo: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
					continue
				}

				err = s.purchaseFuelForTrip(ctx, client, s.RoleData.Location)
				if err != nil {
					log.Printf("%s:%s -- ERROR unable to purchase fuel for trip to trade route sell location: %s\n", s.Username, s.Id, err)
					time.Sleep(60 * time.Second)
				}

				err = s.moveToLocation(ctx, client, s.RoleData.Location)
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

			if err := s.marketplaceRepository.SaveLocationMarketplaceResponses(ctx, s.Location, marketplace); err != nil {
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
