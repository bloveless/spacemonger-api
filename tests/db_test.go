package tests

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"testing"
	"time"

	"spacemonger"
	"spacemonger/spacetraders"

	"github.com/golang-migrate/migrate/v4"
	_ "github.com/golang-migrate/migrate/v4/database/postgres"
	_ "github.com/golang-migrate/migrate/v4/source/file"
	"github.com/jackc/pgx/v4"
	"github.com/jackc/pgx/v4/pgxpool"
	"github.com/stretchr/testify/suite"
)

type DbTestSuite struct {
	suite.Suite
	pgpool *pgxpool.Pool
}

func (suite *DbTestSuite) SetupSuite() {
	connString := "postgres://spacemonger_test:Testing123@localhost:5433/spacemonger_test?sslmode=disable"
	pgpool, err := pgxpool.Connect(context.Background(), connString)
	if err != nil {
		suite.FailNow("Failed trying to create postgres pool", err)
	}

	wd, err := os.Getwd()
	if err != nil {
		log.Fatalf("Unable to determine working directory: %s", err)
	}

	mig, err := migrate.New(fmt.Sprintf("file://%s/migrations", filepath.Dir(wd)), connString)
	if err != nil {
		panic(err)
	}

	err = mig.Up()
	if err != nil && !errors.Is(err, migrate.ErrNoChange) {
		panic(err)
	}

	suite.pgpool = pgpool
}

// Idea adapted from this comment on github
// https://github.com/jackc/pgx/issues/697#issuecomment-604035545
func (suite *DbTestSuite) withTransaction(f func(ctx context.Context, tx pgx.Tx)) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	tx, err := suite.pgpool.Begin(ctx)
	if err != nil {
		suite.FailNow("Unable to start test db transaction", err)
	}

	defer func() {
		err := tx.Rollback(ctx)
		if err != nil {
			suite.FailNow("Unable to rollback transaction. Test database may need to be manually cleaned", err)
		}
	}()

	f(ctx, tx)
}

func (suite *DbTestSuite) TestGetUser() {
	suite.withTransaction(func(ctx context.Context, tx pgx.Tx) {
		rd := spacemonger.RoleData{
			Role:     "Trader",
			System:   "system-1",
			Location: "location-1",
		}
		_, err := tx.Exec(ctx, `
			INSERT INTO daemon_user (username, token, new_ship_role_data)
			VALUES ('test-username', 'test-token', $1);
			`,
			rd,
		)
		if err != nil {
			suite.FailNow("Failed inserting user", err)
		}

		user, err := spacemonger.GetUser(ctx, tx, "test-username")
		if err != nil {
			suite.FailNow("Failed getting user", err)
		}

		suite.NotEmpty(user.Id, "user.Id")
		suite.Equal(user.Username, "test-username", "user.Username")
		suite.Equal(user.Token, "test-token", "user.Token")
		suite.Equal(user.NewShipRoleData, rd)
	})
}

func (suite *DbTestSuite) TestSaveMarketplaceData() {
	suite.withTransaction(func(ctx context.Context, tx pgx.Tx) {
		m := spacetraders.GetLocationMarketplaceResponse{
			Marketplace: []spacetraders.MarketplaceData{{
				Good:                 "METALS",
				VolumePerUnit:        1,
				PurchasePricePerUnit: 10,
				SellPricePerUnit:     9,
				QuantityAvailable:    1000,
			}},
		}
		m2 := spacetraders.GetLocationMarketplaceResponse{
			Marketplace: []spacetraders.MarketplaceData{{
				Good:                 "METALS",
				VolumePerUnit:        2,
				PurchasePricePerUnit: 20,
				SellPricePerUnit:     18,
				QuantityAvailable:    2000,
			}},
		}

		err := spacemonger.SaveLocationMarketplaceResponses(ctx, tx, "location1", m)
		if err != nil {
			suite.Fail("Unable to save marketplace data", err)
		}

		err = spacemonger.SaveLocationMarketplaceResponses(ctx, tx, "location1", m2)
		if err != nil {
			suite.Fail("Unable to save second marketplace data")
		}

		type marketplace struct {
			location             string
			good                 string
			purchasePricePerUnit int
			sellPricePerUnit     int
			volumePerUnit        int
			quantityAvailable    int
		}

		var marketplaceRows []marketplace
		rows, err := tx.Query(ctx, `
			SELECT
				 location
				,good
				,purchase_price_per_unit
				,sell_price_per_unit
				,volume_per_unit
				,quantity_available
			FROM daemon_marketplace;
		`)
		if err != nil {
			suite.Fail("Unable to get marketplace data", err)
		}
		defer rows.Close()

		for rows.Next() {
			m := marketplace{}
			err = rows.Scan(&m.location, &m.good, &m.purchasePricePerUnit, &m.sellPricePerUnit, &m.volumePerUnit, &m.quantityAvailable)
			if err != nil {
				suite.Fail("Unable to read marketplace data row", err)
			}
			marketplaceRows = append(marketplaceRows, m)
		}

		expectedMarketplaceRows := []marketplace{
			{
				location:             "location1",
				good:                 "METALS",
				volumePerUnit:        1,
				purchasePricePerUnit: 10,
				sellPricePerUnit:     9,
				quantityAvailable:    1000,
			},
			{
				location:             "location1",
				good:                 "METALS",
				volumePerUnit:        2,
				purchasePricePerUnit: 20,
				sellPricePerUnit:     18,
				quantityAvailable:    2000,
			},
		}

		suite.Equal(expectedMarketplaceRows, marketplaceRows, "Marketplace data was not as expected")

		var marketplaceLatestRows []marketplace
		latestRows, err := tx.Query(ctx, `
			SELECT
				 location
				,good
				,purchase_price_per_unit
				,sell_price_per_unit
				,volume_per_unit
				,quantity_available
			FROM daemon_marketplace_latest;
		`)
		if err != nil {
			suite.Fail("Unable to get marketplace latest data", err)
		}
		defer latestRows.Close()

		for latestRows.Next() {
			m := marketplace{}
			err = latestRows.Scan(&m.location, &m.good, &m.purchasePricePerUnit, &m.sellPricePerUnit, &m.volumePerUnit, &m.quantityAvailable)
			if err != nil {
				suite.Fail("Unable to read marketplace latest row", err)
			}
			marketplaceLatestRows = append(marketplaceLatestRows, m)
		}

		expectedMarketplaceLatestRows := []marketplace{
			{
				location:             "location1",
				good:                 "METALS",
				volumePerUnit:        2,
				purchasePricePerUnit: 20,
				sellPricePerUnit:     18,
				quantityAvailable:    2000,
			},
		}

		suite.Equal(expectedMarketplaceLatestRows, marketplaceLatestRows, "Marketplace latest data was not as expected")
	})
}

func (suite *DbTestSuite) TestGetRoutesFromLocation() {
	suite.withTransaction(func(ctx context.Context, tx pgx.Tx) {
		_, err := tx.Exec(ctx, `
			INSERT INTO daemon_location (system, system_name, location, location_name, location_type, x, y)
			VALUES ('system1', 'System 1', 'location1', 'Location 1', 'PLANET', 10, 10),
				('system1', 'System 1', 'location2', 'Location 2', 'PLANET', -10, -10);

			INSERT INTO daemon_marketplace_latest (location, good, purchase_price_per_unit, sell_price_per_unit, volume_per_unit, quantity_available)
			VALUES ('location1', 'METALS', 10, 11, 1, 1000),
				('location2', 'METALS', 12, 13, 1, 2000);
		`)

		if err != nil {
			suite.Fail("Error creating market data", err)
		}

		routes, err := spacemonger.GetRoutesFromLocation(ctx, tx, "location1")
		if err != nil {
			suite.Fail("Unable to get routes from location", err)
		}

		expectedRoutes := []spacemonger.DbRoute{
			{
				PurchaseLocation:         "location1",
				PurchaseLocationType:     "PLANET",
				SellLocation:             "location2",
				Good:                     "METALS",
				Distance:                 28.284271247461902,
				PurchaseLocationQuantity: 1000,
				SellLocationQuantity:     2000,
				PurchasePricePerUnit:     10,
				SellPricePerUnit:         13,
				VolumePerUnit:            1,
			},
		}

		suite.Equal(expectedRoutes, routes, "Routes weren't as expected")
	})
}

func (suite *DbTestSuite) TestGetShips() {
	suite.withTransaction(func(ctx context.Context, tx pgx.Tx) {
		rd := spacemonger.RoleData{
			Role:     "Trader",
			System:   "system-1",
			Location: "",
		}

		// rdJson, err := json.Marshal(rd)
		// if err != nil {
		// 	suite.Fail("Unable to marshall role data", err)
		// }

		var userId string
		err := tx.QueryRow(ctx, `
			INSERT INTO daemon_user_ship (
				user_id, ship_id, type, class, max_cargo, loading_speed, speed, manufacturer, plating, weapons, role_data
			) VALUES
			(gen_random_uuid(), 'ship-id-1', 'type-1', 'class-1', 10, 10, 10, 10, 10, 10, $1)
			RETURNING user_id;
			`,
			rd,
		).Scan(&userId)
		if err != nil {
			suite.Fail("Error creating ship data", err)
		}

		ships, err := spacemonger.GetShips(ctx, tx, userId)
		if err != nil {
			suite.Fail("Error getting ships", err)
		}

		fmt.Printf("GetShips: %+v\n", ships)
	})
}

// In order for 'go test' to run this suite, we need to create
// a normal test function and pass our suite to suite.Run
func TestDbTestSuite(t *testing.T) {
	suite.Run(t, new(DbTestSuite))
}
