package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"spacemonger/spacetrader"
	"time"

	"github.com/golang-migrate/migrate/v4"
	_ "github.com/golang-migrate/migrate/v4/database/postgres"
	_ "github.com/golang-migrate/migrate/v4/source/file"
	"github.com/jackc/pgx/v4/pgxpool"
)

type App struct {
	config Config
	dbPool *pgxpool.Pool
}

func NewApp() App {
	config, err := LoadConfig()
	if err != nil {
		panic(err)
	}

	pool, err := pgxpool.Connect(context.Background(), config.PostgresUrl)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Unable to connect to database: %v\n", err)
		os.Exit(1)
	}

	return App{dbPool: pool, config: config}
}

func main() {
	app := NewApp()
	defer app.dbPool.Close()

	m, err := migrate.New("file://migrations", app.config.PostgresUrl)
	if err != nil {
		panic(err)
	}

	err = m.Up()
	if err != nil && !errors.Is(err, migrate.ErrNoChange) {
		panic(err)
	}

	rows, err := app.dbPool.Query(context.Background(), "SELECT schema_name FROM information_schema.schemata")
	if err != nil {
		fmt.Fprintf(os.Stderr, "QueryRow failed: %v\n", err)
		os.Exit(1)
	}

	defer rows.Close()
	for rows.Next() {
		schema := ""

		err = rows.Scan(&schema)
		if err != nil {
			panic(err)
		}

		fmt.Println(schema)
	}

	fmt.Println("Daemon Main")
	c, err := spacetrader.NewClient()
	if err != nil {
		log.Fatalln(err)
	}

	myIp, err := c.GetMyIpAddress()
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Printf("MyIp: %+v\n", myIp)

	status, err := c.GetGameStatus()
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Printf("Game Status: %+v\n", status)

	// claimedUsername, err := c.ClaimUsername("blove-go-test")
	// if err != nil {
	// 	log.Fatalln(err)
	// }

	// claimedUsername := spacemonger.ClaimUsernameResponse{
	// 	Token: "3d472a71-33f9-4752-a38c-761db39425c7",
	// 	User: spacemonger.ClaimUsernameResponseUser{
	// 		Username: "bloveless-dummy-username-test",
	// 		Credits:  0,
	// 		Ships:    []spacemonger.Ship{},
	// 		Loans:    []spacemonger.Loan{},
	// 	},
	// }
	//
	// fmt.Printf("New Username: %+v\n", claimedUsername)

	// username := "blove-go-test"
	token := "c53e4835-d8cc-4579-b7d5-99b1df31bf8e"

	c.SetToken(token)

	myInfo, err := c.GetMyInfo()
	if err != nil {
		log.Fatalf("GetMyInfo error: %+v", err)
	}

	fmt.Printf("GetMyInfo data: %+v\n", myInfo)

	if myInfo.User.Credits == 0 {
		createLoanResponse, err := c.CreateLoan(spacetrader.StartUpLoan)
		if err != nil {
			panic(err)
		}

		fmt.Printf("New Loan: %+v\n", createLoanResponse)
	}

	killSwitch := make(chan struct{}, 1)

	myLoans, err := c.GetMyLoans()
	if err != nil {
		panic(err)
	}

	fmt.Printf("My Loans: %+v\n", myLoans)

	go func() {
		time.Sleep(10 * time.Second)
		killSwitch <- struct{}{}
	}()

	fmt.Println("Waiting for killswitch signal")
	<-killSwitch

	fmt.Println("Received killSwitch... Good Bye")
}
