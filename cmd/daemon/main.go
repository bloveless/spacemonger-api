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

	killSwitch := make(chan struct{})

	// We need to borrow the users client to create the list of known locations
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

	ships := make(chan spacemonger.Ship, 10)

	go func() {
		for ship := range ships {
			ship := ship
			go func() {
				ctx, cancel := context.WithCancel(context.Background())
				defer cancel()

				select {
				case <-killSwitch:
					log.Printf("Caught killswitch. Terminating ship %s\n", ship.Id)
					cancel()
				case err := <-ship.Run(ctx):
					log.Printf("Ship terminated of it's own accord: %+v\n", err)
				}
			}()
		}
	}()

	go func() {
		// User will add all it's ships to the ships channel
		for _, s := range user.Ships {
			newShip, err := spacemonger.ShipFromShipRow(app.dbPool, user, s)
			if err != nil {
				log.Printf("Unexpected error occurred while adding ships to ship channel: %+v", err)
				killSwitch <- struct{}{}
				panic(err)
			}
			ships <- newShip
		}

		// Initially the user will create any ships according to the rules.
		// I.E. While a users credits are greater than 50k buy another JW-MK-I up until a max of 20 ships
		//      Then, auto upgrade ships for example, from a JW-MK-I to GR-MK-I after the user has 200k until a max
		//      of 5 ships have been upgraded... (new ships need to start with a specific role... probably trader)

		for {
			if user.Credits < 50_000 && len(user.Ships) >= 20 {
				break
			}

			newShip, newCredits, err := spacemonger.PurchaseShip(ctx, user, "OE", "JW-MK-I")
			if err != nil {
				panic(err)
			}

			user.Credits = newCredits
			user.Ships = append(user.Ships, newShip)
		}

		// Then wait forever to receive a command from one of it's ships
		for {
			select {
			case msg := <-user.ShipMessages:
				err := user.ProcessShipMessage(msg)
				if errors.Is(err, spacemonger.UnknownShipMessageType) {
					// If we received an unknown ship message type then this is a developer error
					// and we need to add handler code for this
					panic(err)
				}
				// TODO: Are there other errors that need to be handled... right now they are just ignored
			case <-killSwitch:
				log.Printf("%s -- Received kill switch. Terminating", user.Username)
				return
			}
		}
	}()

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

	select {
	case <-sigs:
		log.Println("Caught exit signal. Exiting")
		close(killSwitch)
	case <-killSwitch:
		log.Println("Caught killSwitch. Exiting")
	}

	// TODO: Do I need a waitgroup wait here to wait until all the ships have finished closing after the killSwitch
	//       is triggered
}
