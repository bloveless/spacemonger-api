package main

import (
	"context"
	"errors"
	"fmt"
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
	config spacemonger.Config
	dbPool *pgxpool.Pool
}

func NewApp() App {
	config, err := spacemonger.LoadConfig()
	if err != nil {
		log.Fatalf("Unable to load app config: %s", err)
	}

	pool, err := pgxpool.Connect(context.Background(), config.PostgresUrl)
	if err != nil {
		log.Fatalf("Unable to connect to connect to database: %s", err)
	}

	return App{dbPool: pool, config: config}
}

func purchaseAndAssignShip(ctx context.Context, app App, user *spacemonger.User, systemLocations spacetraders.GetSystemLocationsResponse, shipMessages chan spacemonger.ShipMessage, ships chan spacemonger.Ship) error {
	// TODO: THERE ARE A LOT OF THINGS WRONG WITH THIS FUNCTION... IT WAS DONE HASTILY AND JUST COPIED AND PASTED...
	// FIXME!!!!
	newShip, newCredits, err := spacemonger.PurchaseShip(ctx, *user, "OE", "JW-MK-I")
	if err != nil {
		return fmt.Errorf("unable to purchase ship type \"%s\" in \"%s\": %w", "JW-MK-I", "OE", err)
	}

	user.Credits = newCredits

	log.Printf("%s -- User purchased new ship %+v\n", user.Username, newShip)

	var newCargo []spacemonger.Cargo
	for _, c := range newShip.Cargo {
		newCargo = append(newCargo, spacemonger.Cargo(c))
	}

	// The users first ship will be a trader...
	// TODO: The System "OE" shouldn't be hard coded here
	roleData := spacemonger.RoleData{Role: "Trader", System: "OE"}
	if len(user.Ships) > 0 {
		// After that we will assign each new Scout to an unassigned location
		roleData.Role = "Scout"
		foundScoutLocation := false

		for _, l := range systemLocations.Locations {
			foundAssignedScout := false
			for _, s := range user.Ships {
				if s.RoleData.Role == "Scout" && s.RoleData.Location == l.Symbol {
					log.Printf("%s -- Found scout %s assigned to location %s\n", user.Username, s.Id, l.Symbol)
					foundAssignedScout = true
					break
				}
			}

			if !foundAssignedScout {
				roleData.Location = l.Symbol
				// TODO: The system shouldn't be hard coded here
				roleData.System = "OE"
				foundScoutLocation = true

				log.Printf("%s -- Assigning new scout %s to location %s\n", user.Username, newShip.Id, l.Symbol)

				break
			}
		}

		if !foundScoutLocation {
			// If we were unable to find a scout location assume that every location has a scout assigned and that this ship should be a trader
			roleData.Role = "Trader"
			roleData.System = "OE"
			roleData.Location = ""

			log.Printf("%s:%s -- Unable to find location to assign scout to. Assigning ship as a trader\n", user.Username, newShip.Id)
		}
	}

	// TODO: It is possible that the container exits after buying a ship but before the ship is assigned a role and saved to the DB
	//       The system should be able to correct that by determining that the ship doesn't have a role and performing the same procedure
	//       as above in the InitializeUser method

	s := spacemonger.Ship{
		Username:       user.Username,
		UserId:         user.Id,
		Id:             newShip.Id,
		Type:           newShip.Type,
		Location:       newShip.Location,
		LoadingSpeed:   newShip.LoadingSpeed,
		Speed:          newShip.Speed,
		MaxCargo:       newShip.MaxCargo,
		Cargo:          newCargo,
		SpaceAvailable: newShip.SpaceAvailable,
		RoleData:       roleData,
		Messages:       shipMessages,
	}

	err = spacemonger.SaveShip(ctx, app.dbPool, *user, spacemonger.DbShip{
		UserId:       user.Id,
		ShipId:       newShip.Id,
		Type:         newShip.Type,
		Class:        newShip.Class,
		MaxCargo:     newShip.MaxCargo,
		LoadingSpeed: newShip.LoadingSpeed,
		Speed:        newShip.Speed,
		Manufacturer: newShip.Manufacturer,
		Plating:      newShip.Plating,
		Weapons:      newShip.Weapons,
		RoleData:     roleData,
		Location:     newShip.Location,
	})
	if err != nil {
		return fmt.Errorf("unable to save ship: %w", err)
	}

	ships <- s

	user.Ships = append(user.Ships, s)

	err = spacemonger.SaveUserStats(ctx, app.dbPool, *user)
	if err != nil {
		return fmt.Errorf("unable to save user stats: %w", err)
	}

	return nil
}

func main() {
	app := NewApp()
	defer app.dbPool.Close()

	m, err := migrate.New("file://migrations", app.config.PostgresUrl)
	if err != nil {
		log.Fatalf("Unable to create migrator: %s", err)
	}

	err = m.Up()
	if err != nil && !errors.Is(err, migrate.ErrNoChange) {
		log.Fatalf("Unable to migrate database: %s", err)
	}

	client, err := spacetraders.NewClient()
	if err != nil {
		log.Fatalf("Unable to create client: %s", err)
	}

	ctx := context.Background()
	myIp, err := client.GetMyIpAddress(ctx)
	if err != nil {
		log.Fatalf("Unable to get my ip address: %s", err)
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
		log.Fatalf("Unable to get game status: %s", err)
	}

	log.Printf("Game Status: %+v\n", status)
	log.Printf("App Config %+v\n", app.config)

	user, err := spacemonger.InitializeUser(ctx, client, app.dbPool, app.config.Username, spacemonger.RoleData{Role: "Trader", System: "OE"})
	if err != nil {
		log.Fatalf("Unable to initialize user: %s", err)
	}

	log.Printf("User %+v\n", user)

	// We need to borrow the first users client to create the list of known locations
	// TODO: We only know about OE right now
	system, err := user.Client.GetSystem(ctx, "OE")
	if err != nil {
		log.Fatalf("Unable to get system \"%s\": %s", "OE", err)
	}

	log.Printf("System: %+v\n", system)

	err = spacemonger.SaveSystem(ctx, app.dbPool, system)
	if err != nil {
		log.Fatalf("Unable to save system: %s", err)
	}

	systemLocations, err := user.Client.GetSystemLocations(ctx, "OE")
	if err != nil {
		log.Fatalf("Unable to get system locations: %s", err)
	}

	log.Printf("System Locations: %+v\n", systemLocations)

	for _, location := range systemLocations.Locations {
		if err = spacemonger.SaveLocation(ctx, app.dbPool, spacemonger.DbLocation{
			System:       "OE",
			Location:     location.Symbol,
			LocationName: location.Name,
			Type:         location.Type,
			X:            location.X,
			Y:            location.Y,
		}); err != nil {
			log.Fatalf("Unable to save location: %s", err)
		}
	}

	// When implementing the ship the ship will have a few layers of strategy. Early on the ship won't know anything
	// about the market so it will just buy a good from the location it is at and move to the closest location to sell
	// that good. The ship will harvest market data after it arrives at each location. After the ship harvests data
	// from both locations then it might be able to make a profitable trade. Try and expand this algorithm to the 3 or 4
	// closest locations and pick trade routes within those

	exit := make(chan struct{}, 1)
	ships := make(chan spacemonger.Ship, 3)

	go func() {
		for ship := range ships {
			ship := ship
			go func() {
				log.Printf("%s:%s -- Starting process for ship\n", ship.Username, ship.Id)
				ship.Run(ctx, app.config, app.dbPool, user.Client)
			}()
		}
	}()

	go func() {
		for {
			shipMessages := make(chan spacemonger.ShipMessage, 10)

			// First the user needs to add all of it's ships to the ships channel
			log.Printf("%s -- Users ships %+v\n", user.Username, user.Ships)
			for _, s := range user.Ships {
				s.Messages = shipMessages
				ships <- s
			}

			// Special boot up instructions are to buy as many ships as possible before we start running
			// this is because we have to have ships docked in a location in order to buy them but when we
			// are first purchasing a ship we can purchase it from anywhere... which means we will have ships
			// at that location to buy a bunch of ships.
			if len(user.Ships) == 0 {
				for user.Credits > 50_000 && len(user.Ships) < 20 {
					// TODO: It seems like the user credits aren't accurate here... probably due to the gross
					//       purchaseAndAssignShip function here
					err := purchaseAndAssignShip(ctx, app, &user, systemLocations, shipMessages, ships)
					if err != nil {
						log.Printf("%s -- ERROR unable to initially purchase and assign ships: %s\n", user.Username, err)
					}
				}
			}

			for {
				// Next the user needs to process any rules that need processing.
				// I.E. If the user has > 50k credits then buy as many cheap ships for probes as possible
				if user.Credits > 50_000 && len(user.Ships) < 20 {
					err := purchaseAndAssignShip(ctx, app, &user, systemLocations, shipMessages, ships)
					if err != nil {
						log.Printf("%s -- ERROR unable to initially purchase and assign ships: %s\n", user.Username, err)
					}
				}

				// Wait for a message to come back from a ship and run the rules again
				message := <-shipMessages
				if message.Type == spacemonger.ShipMessageUpdateCredits {
					log.Printf("%s -- Received update credits message from ship %+v", user.Username, message)
					user.Credits = message.NewCredits

					err = spacemonger.SaveUserStats(ctx, app.dbPool, user)
					if err != nil {
						log.Printf("%s - ERROR Unable to save user stats: %s", user.Username, err)
					}
				}
			}
		}
	}()

	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGINT, syscall.SIGTERM)

	select {
	case <-signals:
		log.Println("Caught exit signal. Exiting")
		close(exit)
	case <-exit:
		log.Println("Caught exit. Exiting")
	}
}
