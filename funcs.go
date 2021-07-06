package spacemonger

import (
	"context"
	"fmt"
	"log"
	"regexp"
	"strconv"
	"time"

	"spacemonger/spacetraders"
)

type FlightPlan struct {
	Id                     string
	ShipId                 string
	FuelConsumed           int
	FuelRemaining          int
	TimeRemainingInSeconds int
	CreatedAt              time.Time
	ArrivesAt              time.Time
	TerminatedAt           time.Time
	Destination            string
	Origin                 string
	Distance               int
}

func CreateFlightPlan(ctx context.Context, client spacetraders.AuthorizedClient, conn DbConn, s *Ship, destination string) (FlightPlan, error) {
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

	err = SaveFlightPlan(ctx, conn, s.UserId, DbFlightPlan{
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

func CreatePurchaseOrder(ctx context.Context, client spacetraders.AuthorizedClient, conn DbConn, s *Ship, good string, quantity int) (int, error) {
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

		err = SaveTransaction(ctx, conn, DbTransaction{
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

func CreateSellOrder(ctx context.Context, client spacetraders.AuthorizedClient, conn DbConn, s *Ship, good string, quantity int) (int, error) {
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

		err = SaveTransaction(ctx, conn, DbTransaction{
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

func GetAdditionalFuelRequiredForTrip(ctx context.Context, client spacetraders.AuthorizedClient, conn DbConn, s Ship, destination string) (int, error) {
	currentFuel := 0
	for _, c := range s.Cargo {
		if c.Good == GoodFuel {
			currentFuel += c.Quantity
		}
	}

	log.Printf("%s:%s -- Ship currently has %d fuel\n", s.Username, s.Id, currentFuel)

	dbFuelRequired, err := GetFuelRequired(ctx, conn, s.Location, destination, s.Type)
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

// PurchaseFastestShip will attempt to purchase a new ship for the user. If no ship was able to be purchased then the
// original unmodified user will be returned along with the error.
func PurchaseFastestShip(ctx context.Context, u User, system string) (spacetraders.Ship, int, error) {
	availableShips, err := u.Client.GetShipsForSale(ctx)
	if err != nil {
		return spacetraders.Ship{}, 0, err
	}

	currentShips, err := u.Client.GetMyShips(ctx)
	if err != nil {
		return spacetraders.Ship{}, 0, err
	}

	dockedShipLocations := make(map[string]bool)
	for _, ship := range currentShips.Ships {
		if ship.Location != "" {
			dockedShipLocations[ship.Location] = true
		}
	}

	log.Printf("%s -- Docked ship locations are %v\n", u.Username, dockedShipLocations)
	log.Printf("%s -- User has %d ships\n", u.Username, len(u.Ships))
	log.Printf("%s -- Ships available for purchase %+v\n", u.Username, availableShips)

	if len(u.Ships) > 0 && len(dockedShipLocations) == 0 {
		log.Printf("%s -- No docked ships found. Unable to purchase new ship. Will retry later\n", u.Username)
		return spacetraders.Ship{}, 0, nil
	}

	fastestShipSpeed := 0
	fastestShipPrice := 0
	fastestShipLocation := ""
	fastestShipType := ""
	foundShip := false

	for _, availableShip := range availableShips.ShipsForSale {
		for _, purchaseLocation := range availableShip.PurchaseLocations {
			// users can only purchase ships at locations where they have a ship docked...
			// unless they currently don't have any ships
			if _, ok := dockedShipLocations[purchaseLocation.Location]; !ok && len(u.Ships) > 0 {
				continue
			}

			// TODO: Handle restricted goods better. Right now I just ignore any ships that are restricted
			//       to specific goods
			if len(availableShip.RestrictedGoods) > 0 {
				continue
			}

			if u.Credits < purchaseLocation.Price {
				continue
			}

			if availableShip.Speed < fastestShipSpeed {
				continue
			}

			if purchaseLocation.System != system {
				continue
			}

			foundShip = true
			fastestShipSpeed = availableShip.Speed
			fastestShipType = availableShip.Type
			fastestShipLocation = purchaseLocation.Location
			fastestShipPrice = purchaseLocation.Price
		}
	}

	if !foundShip {
		return spacetraders.Ship{}, 0, fmt.Errorf("%s -- unable to find a ship for the user to purchase", u.Username)
	}

	log.Printf("%s -- Buying ship %s for %d at location %s\n", u.Username, fastestShipType, fastestShipPrice, fastestShipLocation)
	s, err := u.Client.PurchaseShip(ctx, fastestShipLocation, fastestShipType)
	if err != nil {
		return spacetraders.Ship{}, 0, err
	}

	return s.Ship, s.Credits, nil
}

// PurchaseShip will attempt to purchase a new ship for the user. If no ship was able to be purchased then the
// original unmodified user will be returned along with the error.
func PurchaseShip(ctx context.Context, u User, system string, shipType string) (spacetraders.Ship, int, error) {
	availableShips, err := u.Client.GetShipsForSale(ctx)
	if err != nil {
		return spacetraders.Ship{}, 0, err
	}

	currentShips, err := u.Client.GetMyShips(ctx)
	if err != nil {
		return spacetraders.Ship{}, 0, err
	}

	dockedShipLocations := make(map[string]bool)
	for _, ship := range currentShips.Ships {
		if ship.Location != "" {
			dockedShipLocations[ship.Location] = true
		}
	}

	if len(u.Ships) > 0 && len(dockedShipLocations) == 0 {
		log.Printf("%s -- No docked ships found. Unable to purchase new ship. Will retry later\n", u.Username)
		return spacetraders.Ship{}, 0, nil
	}

	foundShip := false
	shipPrice := 0
	shipLocation := ""

	for _, availableShip := range availableShips.ShipsForSale {
		for _, purchaseLocation := range availableShip.PurchaseLocations {
			// users can only purchase ships at locations where they have a ship docked...
			// unless they currently don't have any ships
			if _, ok := dockedShipLocations[purchaseLocation.Location]; !ok && len(u.Ships) > 0 {
				continue
			}

			// TODO: Handle restricted goods better. Right now I just ignore any ships that are restricted
			//       to specific goods
			if len(availableShip.RestrictedGoods) > 0 {
				continue
			}

			if purchaseLocation.System != system {
				continue
			}

			if availableShip.Type != shipType {
				continue
			}

			foundShip = true
			shipPrice = purchaseLocation.Price
			shipLocation = purchaseLocation.Location
		}
	}

	if !foundShip {
		log.Printf("%s -- Docked ship locations are %v\n", u.Username, dockedShipLocations)
		log.Printf("%s -- User has %d ships\n", u.Username, len(u.Ships))
		log.Printf("%s -- Ships available for purchase %+v\n", u.Username, availableShips)

		return spacetraders.Ship{}, 0, fmt.Errorf("%s -- unable to find a ship for the user to purchase", u.Username)
	}

	log.Printf("%s -- Buying ship %s for %d at location %s\n", u.Username, shipType, shipPrice, shipLocation)
	s, err := u.Client.PurchaseShip(ctx, shipLocation, shipType)
	if err != nil {
		log.Printf("%s -- Error puchasing ship %v", u.Username, err)
		return spacetraders.Ship{}, 0, err
	}

	log.Printf("%s -- User purchased ship %+v\n", u.Username, s)

	return s.Ship, s.Credits, nil
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

func GetBestTradingRoute(ctx context.Context, conn DbConn, s Ship) (Route, error) {
	log.Printf("Getting routes for ship from location \"%s\"\n", s.Location)
	dbRoutes, err := GetRoutesFromLocation(ctx, conn, s.Location)
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

		if route.SellLocation == "OE-XV-91-2" {
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
