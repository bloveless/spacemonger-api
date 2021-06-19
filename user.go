package spacemonger

import (
	"context"
	"errors"
	"fmt"
	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/pgxpool"
	"log"
	"spacemonger/spacetrader"
	"strings"
)

type UserMachine struct {
	client spacetrader.Client
	user   User
}

func GetOrCreateUser(pool *pgxpool.Pool, username string, newShipAssignment string) (UserMachine, error) {
	// Get user from DB
	ctx := context.Background()
	client, err := spacetrader.NewClient()
	if err != nil {
		return UserMachine{}, err
	}

	dbUser, err := GetUser(ctx, pool, username)
	if errors.Is(err, pgx.ErrNoRows) {
		log.Printf("Creating new user: %s", username)

		claimedUsername, err := client.ClaimUsername(username)
		if err != nil {
			return UserMachine{}, err
		}

		fmt.Printf("claimedUsername: %+v", claimedUsername)


		dbUser, err = SaveUser(ctx, pool, User{
			Username:          username,
			Token:             claimedUsername.Token,
			NewShipAssignment: newShipAssignment,
			NewShipSystem:     "OE", // TODO: Don't hard code this to OE
		})
		if err != nil {
			return UserMachine{}, err
		}

		log.Printf("New user persisted: %s", username)
	}
	if err != nil {
		return UserMachine{}, fmt.Errorf("unknown error occurred: %w", err)
	}

	fmt.Printf("User: %+v\n", dbUser)

	// now that we have a valid user we can populate all the information about the user
	client.SetToken(dbUser.Token)
	info, err := client.GetMyInfo()
	if err != nil {
		return UserMachine{}, err
	}
	dbUser.Credits = info.User.Credits

	loans, err := client.GetMyLoans()
	if err != nil {
		return UserMachine{}, err
	}
	dbUser.Loans = loans.Loans

	outstandingLoans := 0
	for _, l := range loans.Loans {
		if !strings.Contains(l.Status, "PAID") {
			outstandingLoans += 1
		}
	}
	dbUser.OutstandingLoans = outstandingLoans

	ships, err := client.GetMyShips()
	if err != nil {
		return UserMachine{}, err
	}
	// TODO: Process ships into ship machines and add them to the user

	// TODO: Save ships to DB
	for _, ship := range ships.Ships {
		err := SaveShip(ctx, pool, dbUser.Id, ship)
		if err != nil {
			return UserMachine{}, err
		}
	}

	fmt.Printf("Info: %+v\n", info)
	fmt.Printf("Ships: %+v\n", ships)
	fmt.Printf("Loans: %+v\n", loans)

	return UserMachine{
		client: client,
		user:   dbUser,
	}, nil
}
