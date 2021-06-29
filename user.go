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

	ships, err := u.Client.GetMyShips(ctx)
	if err != nil {
		return User{}, err
	}

	for _, ship := range ships.Ships {
		sr := ShipRow{
			UserId:       u.Id,
			ShipId:       ship.Id,
			Type:         ship.Type,
			Class:        ship.Class,
			MaxCargo:     ship.MaxCargo,
			LoadingSpeed: ship.LoadingSpeed,
			Speed:        ship.Speed,
			Manufacturer: ship.Manufacturer,
			Plating:      ship.Plating,
			Weapons:      ship.Weapons,
			// If this is a new ship then the new user role data will be used, if the ship exists it will not be altered
			RoleData:     dbUser.NewShipRoleData,
			Location:     ship.Location,
		}

		if err := SaveShip(ctx, pool, dbUser.Id, sr); err != nil {
			return User{}, err
		}
	}

	shipRows, err := GetShips(ctx, pool, u.Id)
	if err != nil {
		return User{}, err
	}
	u.Ships = shipRows

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
