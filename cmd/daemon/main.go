package main

import (
	"context"
	"errors"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"spacemonger"
	"spacemonger/spacetraders"

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
		log.Printf("Unable to connect to database: %v\n", err)
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

	client, err := spacetraders.NewClient()
	if err != nil {
		log.Fatalf("Unable to create client: %v", err)
	}

	ctx := context.Background()
	myIp, err := client.GetMyIpAddress(ctx)
	if err != nil {
		log.Fatalln(err)
	}

	log.Printf("MyIp: %+v\n", myIp)

	status, err := client.GetGameStatus(ctx)
	if errors.Is(err, spacetraders.MaintenanceModeError) {
		for {
			log.Println("Detected SpaceTraders API in maintenance mode (status code 503). Sleeping for 60 seconds and trying again")
			time.Sleep(60 * time.Second)

			_, err = client.GetGameStatus(ctx)
			if err == nil || !errors.Is(err, spacetraders.MaintenanceModeError) {
				break
			}
		}
	}
	if err != nil {
		log.Fatalln(err)
	}
	log.Printf("Game Status: %+v\n", status)

	user, err := spacemonger.InitializeUser(ctx, client, app.dbPool, app.config.Username, spacemonger.RoleData{Role: "Trader", System: "OE"})
	if err != nil {
		panic(err)
	}
	log.Printf("User %+v\n", user)

	// We need to borrow the first users client to create the list of known locations
	// TODO: We only know about OE right now
	system, err := user.Client.GetSystem(ctx, "OE")
	if err != nil {
		panic(err)
	}

	log.Printf("System: %+v\n", system)

	err = spacemonger.SaveSystem(ctx, app.dbPool, system)
	if err != nil {
		panic(err)
	}

	systemLocations, err := user.Client.GetSystemLocations(ctx, "OE")
	if err != nil {
		panic(err)
	}

	log.Printf("System Locations: %+v\n", systemLocations)

	for _, location := range systemLocations.Locations {
		err = spacemonger.SaveLocation(ctx, app.dbPool, spacemonger.LocationRow{
			System:       "OE",
			Location:     location.Symbol,
			LocationName: location.Name,
			Type:         location.Type,
			X:            location.X,
			Y:            location.Y,
		})

		if err != nil {
			panic(err)
		}
	}

	// When implementing the ship the ship will have a few layers of strategy. Early on the ship won't know anything
	// about the market so it will just buy a good from the location it is at and move to the closest location to sell
	// that good. The ship will harvest market data after it arrives at each location. After the ship harvests data
	// from both locations then it might be able to make a profitable trade. Try and expand this algorithm to the 3 or 4
	// closest locations and pick trade routes within those

	// our first implementation will be a simple one just loop through all the users ships one at a time and process their
	// next step

	exit := make(chan struct{}, 1)
	ships := make(chan spacemonger.Ship, 3)

	go func() {
		for ship := range ships {
			ship := ship
			go func() {
				ship.Run(ctx, app.dbPool, user.Client)
			}()
		}
	}()

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

	select {
	case <-sigs:
		log.Println("Caught exit signal. Exiting")
		close(exit)
	case <-exit:
		log.Println("Caught exit. Exiting")
	}

	// TODO: Do I need a waitgroup wait here to wait until all the ships have finished closing after the killSwitch
	//       is triggered
}
