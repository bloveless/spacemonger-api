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
	Ships            []ShipRow
	Loans            []spacetraders.Loan
	OutstandingLoans int
	Credits          int
	Client           spacetraders.AuthorizedClient
	ShipMessages     chan ShipMessage
}

// InitializeUser will get or create the user in the db and get the user ready to play. This means that if the user has
// no money attempt to take out a loan. Maybe if the user doesn't have any ships then we should purchase a ship.
func InitializeUser(ctx context.Context, client spacetraders.Client, pool *pgxpool.Pool, username string, newShipRoleData RoleData) (User, error) {
	// Get user from DB
	dbUser, err := GetUser(ctx, pool, username)
	if errors.Is(err, pgx.ErrNoRows) {
		log.Printf("Creating new user: %s\n", username)

		claimedUsername, err := client.ClaimUsername(ctx, username)
		if err != nil {
			return User{}, err
		}

		log.Printf("ClaimedUsername: %+v\n", claimedUsername)

		dbUser, err = SaveUser(ctx, pool, UserRow{
			Username:        username,
			Token:           claimedUsername.Token,
			NewShipRoleData: newShipRoleData,
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

	// ships, err := u.Client.GetMyShips(ctx)
	// if err != nil {
	// 	return User{}, err
	// }
	//
	// for _, ship := range ships.Ships {
	// 	if err := SaveShip(ctx, pool, dbUser.Id, ship); err != nil {
	// 		return User{}, err
	// 	}
	// }

	ships, err := GetShips(ctx, pool, u.Id)
	if err != nil {
		return User{}, err
	}
	u.Ships = ships

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
		newShip, newCredits, err := PurchaseFastestShip(ctx, u, newShipRoleData.System)
		if err != nil {
			// This is an interesting case because in general if we can't purchase a ship it's no big deal and we'll
			// try again later... but here the user has no ships and wasn't able to buy one... so the user can't operate
			return u, err
		}

		log.Printf("New Ship: %+v\n", newShip)
		log.Printf("New credits: %d\n", newCredits)

		// TODO: We need to save this ship to the db then add this ship to the users ship array
		s := ShipRow{
			UserId:       u.Id,
			ShipId:       newShip.Id,
			Type:         newShip.Type,
			Class:        newShip.Class,
			MaxCargo:     newShip.MaxCargo,
			LoadingSpeed: newShip.LoadingSpeed,
			Speed:        newShip.Speed,
			Manufacturer: newShip.Manufacturer,
			Plating:      newShip.Plating,
			Weapons:      newShip.Weapons,
			RoleData:     newShipRoleData,
		}

		u.Ships = append(u.Ships, s)
		u.Credits = newCredits

		err = SaveShip(ctx, pool, u.Id, s)
		if err != nil {
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
