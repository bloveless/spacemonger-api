package spacemonger

import (
	"context"
	"errors"
	"fmt"
	"log"
	"strings"

	"spacemonger/spacetraders"

	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/pgxpool"
)

type User struct {
	Id               string
	Token            string
	Username         string
	Ships            []spacetraders.Ship
	Loans            []spacetraders.Loan
	OutstandingLoans int
	Credits          int
	Client           spacetraders.AuthorizedClient
	ShipMessages     chan ShipMessage
}

// InitializeUser will get or create the user in the db and get the user ready to play. This means that if the user has
// no money attempt to take out a loan. Maybe if the user doesn't have any ships then we should purchase a ship.
func InitializeUser(ctx context.Context, client spacetraders.Client, pool *pgxpool.Pool, username string, newShipAssignment string) (User, error) {
	// Get user from DB
	dbUser, err := GetUser(ctx, pool, username)
	if errors.Is(err, pgx.ErrNoRows) {
		log.Printf("Creating new user: %s\n", username)

		claimedUsername, err := client.ClaimUsername(ctx, username)
		if err != nil {
			return User{}, err
		}

		log.Printf("ClaimedUsername: %+v\n", claimedUsername)

		dbUser, err = SaveUser(ctx, pool, DbUser{
			Username:          username,
			Token:             claimedUsername.Token,
			NewShipAssignment: newShipAssignment,
			NewShipSystem:     "OE", // TODO: This shouldn't be hard coded to OE
		})
		if err != nil {
			return User{}, err
		}

		log.Printf("New user persisted: %s\n", username)
	}
	if err != nil && !errors.Is(err, pgx.ErrNoRows) {
		return User{}, fmt.Errorf("unknown error occurred: %w", err)
	}

	authorizedClient, err := spacetraders.NewAuthorizedClient(client, dbUser.Token)
	if err != nil {
		return User{}, err
	}

	u := User{
		Id:           dbUser.Id,
		Token:        dbUser.Token,
		Username:     dbUser.Username,
		Client:       authorizedClient,
		ShipMessages: make(chan ShipMessage, 10),
	}

	info, err := u.Client.GetMyInfo(ctx)
	if err != nil {
		return User{}, err
	}
	u.Credits = info.User.Credits

	loans, err := u.Client.GetMyLoans(ctx)
	if err != nil {
		return User{}, err
	}
	u.Loans = loans.Loans

	outstandingLoans := 0
	for _, l := range loans.Loans {
		if !strings.Contains(l.Status, "PAID") {
			outstandingLoans += 1
		}
	}
	u.OutstandingLoans = outstandingLoans

	ships, err := u.Client.GetMyShips(ctx)
	if err != nil {
		return User{}, err
	}

	for _, ship := range ships.Ships {
		if err := SaveShip(ctx, pool, dbUser.Id, ship); err != nil {
			return User{}, err
		}
	}

	u.Ships = ships.Ships

	if u.Credits == 0 {
		createLoanResponse, err := u.Client.CreateLoan(ctx, spacetraders.StartUpLoan)
		if err != nil {
			return User{}, err
		}

		u.Loans = append(u.Loans, createLoanResponse.Loan)
		u.Credits = createLoanResponse.Credits

		outstandingLoans := 0
		for _, l := range u.Loans {
			if !strings.Contains(l.Status, "PAID") {
				outstandingLoans += 1
			}
		}
		u.OutstandingLoans = outstandingLoans

		log.Printf("New Loan: %+v\n", createLoanResponse)
	}

	if len(u.Ships) == 0 {
		u, err = PurchaseFastestShip(ctx, u, "OE") // TODO: this shouldn't be hard coded to OE
		if err != nil {
			// This is an interesting case because in general if we can't purchase a ship it's no big deal and we'll
			// try again later... but here the user has no ships and wasn't able to buy one... so the user can't operate
			return u, err
		}
	}

	return u, nil
}

func (u *User) ProcessShipMessage(m ShipMessage) error {
	if m.Type == UpdateCredits {
		log.Printf("%s -- Updating credits to %d", u.Username, m.NewCredits)
		u.Credits = m.NewCredits
		return nil
	}

	return UnknownShipMessageType
}

// PurchaseFastestShip will attempt to purchase a new ship for the user. If no ship was able to be purchased then the
// original unmodified user will be returned along with the error.
func PurchaseFastestShip(ctx context.Context, u User, system string) (User, error) {
	availableShips, err := u.Client.GetShipsForSale(ctx)
	if err != nil {
		return u, err
	}

	dockedShipLocations := make(map[string]bool)
	for _, ship := range u.Ships {
		if ship.Location != "" {
			dockedShipLocations[ship.Location] = true
		}
	}

	log.Printf("%s -- Docked ship locations are %v\n", u.Username, dockedShipLocations)
	log.Printf("%s -- User has %d ships\n", u.Username, len(u.Ships))
	log.Printf("%s -- Ships available for purchase %+v\n", u.Username, availableShips)

	if len(u.Ships) > 0 && len(dockedShipLocations) == 0 {
		log.Printf("%s -- No docked ships found. Unable to purchase new ship. Will retry later\n", u.Username)
		return u, nil
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
			fastestShipType = availableShip.ShipType
			fastestShipLocation = purchaseLocation.Location
			fastestShipPrice = purchaseLocation.Price
		}
	}

	if foundShip == false {
		return u, fmt.Errorf("%s -- unable to find a ship for the user to purchase", u.Username)
	}

	log.Printf("%s -- Buying ship %s for %d at location %s\n", u.Username, fastestShipType, fastestShipPrice, fastestShipLocation)
	s, err := u.Client.PurchaseShip(ctx, fastestShipLocation, fastestShipType)
	if err != nil {
		return u, err
	}

	u.Ships = append(u.Ships, s.Ship)
	u.Credits = s.Credits

	return u, nil
}

// func PurchaseLargestShip(u User) (User, error) {
//
// }
