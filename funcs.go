package spacemonger

import (
	"context"
	"fmt"
	"log"
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
			//       to specific goods... unless we are requesting a Tiddalik
			if shipType != "TD-MK-I" && len(availableShip.RestrictedGoods) > 0 {
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
