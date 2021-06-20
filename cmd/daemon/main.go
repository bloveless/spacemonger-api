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
	"spacemonger/spacetrader"

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

	log.Println("Daemon Main")
	c, err := spacetrader.NewClient()
	if err != nil {
		log.Fatalln(err)
	}

	ctx := context.Background()
	myIp, err := c.GetMyIpAddress(ctx)
	if err != nil {
		log.Fatalln(err)
	}

	log.Printf("MyIp: %+v\n", myIp)

	status, err := c.GetGameStatus(ctx)
	if errors.Is(err, spacetrader.MaintenanceModeError) {
		for {
			log.Println("Detected SpaceTraders API in maintenance mode (status code 503). Sleeping for 60 seconds and trying again")
			time.Sleep(60*time.Second)

			_, err = c.GetGameStatus(ctx)
			if err == nil || !errors.Is(err, spacetrader.MaintenanceModeError) {
				break
			}
		}
	}
	if err != nil {
		log.Fatalln(err)
	}
	log.Printf("Game Status: %+v\n", status)

	user, err := spacemonger.InitializeUser(ctx, app.dbPool, fmt.Sprintf("%s-main", app.config.UsernameBase), "trader")
	if err != nil {
		panic(err)
	}
	log.Printf("User %+v\n", user)

	killSwitch := make(chan struct{})

	// When implementing the ship the ship will have a few layers of strategy. Early on the ship won't know anything
	// about the market so it will just buy a good from the location it is at and move to the closest location to sell
	// that good. The ship will harvest market data after it arrives at each location. After the ship harvests data
	// from both locations then it might be able to make a profitable trade. Try and expand this algorithm to the 3 or 4
	// closest locations and pick trade routes within those

	ships := make(chan spacemonger.Ship, 1)

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

	for _, s := range user.Ships {
		ships <- spacemonger.NewShip(app.dbPool, user, s)
	}

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

	select {
	case <-sigs:
		log.Println("Caught exit signal. Exiting")
		close(killSwitch)
	case <- killSwitch:
		log.Println("Caught killSwitch. Exiting")
	}

	// TODO: Do I need a waitgroup wait here to wait until all the ships have finished closing after the killSwitch
	//       is triggered
}
