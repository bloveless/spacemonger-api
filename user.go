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
	Ships            []Ship
	Loans            []Loan
	OutstandingLoans int
	Credits          int
	NewShipRoleData  RoleData
	// TODO: Is this wrong?
	Client spacetraders.AuthorizedClient
}

// InitializeUser will get or create the user in the db and get the user ready to play. This means that if the user has
// no money attempt to take out a loan. Maybe if the user doesn't have any ships then we should purchase a ship.
func InitializeUser(ctx context.Context, client spacetraders.Client, pool *pgxpool.Pool, username string, newShipRoleData RoleData) (User, error) {
	// Get user from DB
	user, err := GetUser(ctx, pool, username)
	if errors.Is(err, pgx.ErrNoRows) {
		log.Printf("Creating new user: %s\n", username)

		claimedUsername, err := client.ClaimUsername(ctx, username)
		if err != nil {
			return User{}, err
		}

		log.Printf("ClaimedUsername: %+v\n", claimedUsername)

		user, err = SaveUser(ctx, pool, User{
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

	authorizedClient, err := spacetraders.NewAuthorizedClient(client, user.Token)
	if err != nil {
		return User{}, err
	}
	user.Client = authorizedClient

	info, err := user.Client.GetMyInfo(ctx)
	if err != nil {
		return User{}, err
	}
	user.Credits = info.User.Credits

	log.Printf("%s -- User Credits %d\n", user.Username, user.Credits)

	apiLoans, err := user.Client.GetMyLoans(ctx)
	if err != nil {
		return User{}, err
	}

	var loans []Loan
	for _, l := range apiLoans.Loans {
		loans = append(loans, Loan(l))
	}

	user.Loans = loans

	log.Printf("%s -- User Loans %+v\n", user.Username, user.Loans)

	outstandingLoans := 0
	for _, l := range loans {
		if !strings.Contains(l.Status, "PAID") {
			outstandingLoans += 1
		}
	}
	user.OutstandingLoans = outstandingLoans

	log.Printf("%s -- User Outstanding Loans %d\n", user.Username, user.OutstandingLoans)

	apiShips, err := user.Client.GetMyShips(ctx)
	if err != nil {
		return User{}, err
	}

	log.Printf("%s -- User Api Ships %+v\n", user.Username, apiShips)

	dbShips, err := GetShips(ctx, pool, user.Id)
	if err != nil {
		return User{}, err
	}

	log.Printf("%s -- User Db Ships %+v\n", user.Username, dbShips)

	// Save/Update any ships that the user might not have saved in the DB yet
	for _, apiShip := range apiShips.Ships {
		shipExists := false
		roleData := newShipRoleData
		for _, dbShip := range dbShips {
			if apiShip.Id == dbShip.ShipId {
				roleData = dbShip.RoleData
				shipExists = true

				log.Printf("%s:%s -- Ship exists and has role data %+v\n", user.Username, dbShip.ShipId, roleData)

				break
			}
		}

		if !shipExists {
			dbShip := DbShip{
				UserId:       user.Id,
				ShipId:       apiShip.Id,
				Type:         apiShip.Type,
				Class:        apiShip.Class,
				MaxCargo:     apiShip.MaxCargo,
				LoadingSpeed: apiShip.LoadingSpeed,
				Speed:        apiShip.Speed,
				Manufacturer: apiShip.Manufacturer,
				Plating:      apiShip.Plating,
				Weapons:      apiShip.Weapons,
				// If this is a new ship then the new user role data will be used, if the ship exists it will not be altered
				RoleData: roleData,
				Location: apiShip.Location,
			}

			log.Printf("%s:%s -- Ship did not exist and was created with role data %+v\n", user.Username, dbShip.ShipId, roleData)

			err = SaveShip(ctx, pool, user, dbShip)
			if err != nil {
				return User{}, err
			}
		}

		s := Ship{
			Username:     user.Username,
			Id:           apiShip.Id,
			Location:     apiShip.Location,
			LoadingSpeed: apiShip.LoadingSpeed,
			MaxCargo:     apiShip.MaxCargo,
			RoleData:     roleData,
		}

		for _, cargo := range apiShip.Cargo {
			s.Cargo = append(s.Cargo, Cargo(cargo))
		}

		user.Ships = append(user.Ships, s)
	}

	log.Printf("%s -- User Ships %+v\n", user.Username, user.Ships)

	if user.Credits == 0 {
		createLoanResponse, err := user.Client.CreateLoan(ctx, spacetraders.StartUpLoan)
		if err != nil {
			return User{}, err
		}

		log.Printf("%s -- User took out new loan %+v\n", user.Username, createLoanResponse)

		user.Loans = append(user.Loans, Loan(createLoanResponse.Loan))
		user.Credits = createLoanResponse.Credits

		log.Printf("%s -- User Loans %+v\n", user.Username, user.Loans)
		log.Printf("%s -- User Credits %+v\n", user.Username, user.Credits)

		outstandingLoans := 0
		for _, l := range user.Loans {
			if !strings.Contains(l.Status, "PAID") {
				outstandingLoans += 1
			}
		}
		user.OutstandingLoans = outstandingLoans

		log.Printf("%s -- User Outstanding Loans %d\n", user.Username, user.OutstandingLoans)
	}

	return user, nil
}

func (u *User) ProcessShipMessage(m ShipMessage) error {
	if m.Type == UpdateCredits {
		log.Printf("%s -- Updating credits to %d", u.Username, m.NewCredits)
		u.Credits = m.NewCredits
		return nil
	}

	return UnknownShipMessageType
}
