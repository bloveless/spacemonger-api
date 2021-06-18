package tests

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"spacemonger"
	"testing"
	"time"

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
	pgpool, err := pgxpool.Connect(context.Background(), fmt.Sprintf("postgres://spacemonger_test:Testing123@localhost:5433/spacemonger_test?sslmode=disable"))
	if err != nil {
		suite.FailNow("Failed trying to create postgres pool", err)
	}

	wd, err := os.Getwd()
	if err != nil {
		log.Fatalf("Unable to determine working directory: %s", err)
	}

	mig, err := migrate.New(fmt.Sprintf("file://%s/migrations", filepath.Dir(wd)), fmt.Sprintf("postgres://spacemonger_test:Testing123@localhost:5433/spacemonger_test?sslmode=disable"))
	if err != nil {
		panic(err)
	}

	err = mig.Up()
	if err != nil && !errors.Is(err, migrate.ErrNoChange) {
		panic(err)
	}

	suite.pgpool = pgpool
}

func (suite *DbTestSuite) withTransation(f func(ctx context.Context, tx pgx.Tx)) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	tx, err := suite.pgpool.Begin(ctx)
	defer tx.Rollback(ctx)
	if err != nil {
		suite.FailNow("Failed to start connection", err)
	}

	f(ctx, tx)
}

func (suite *DbTestSuite) TestGetUser() {
	suite.withTransation(func(ctx context.Context, tx pgx.Tx) {
		rows, err := tx.Query(ctx, `
			INSERT INTO daemon_user (username, token, new_ship_assignment, new_ship_system)
			VALUES ('test-username', 'test-token', 'test-assignment', 'test-system');
		`)
		if err != nil {
			suite.FailNow("Failed inserting user", err)
		}
		rows.Close()

		user, err := spacemonger.GetUser(ctx, tx, "test-username")
		if err != nil {
			suite.FailNow("Failed getting user", err)
		}

		suite.NotEmpty(user.Id, "user.Id")
		suite.Equal(user.Username, "test-username", "user.Username")
		suite.Equal(user.Token, "test-token", "user.Token")
		suite.Equal(user.NewShipAssignment, "test-assignment", "user.NewShipAssignment")
		suite.Equal(user.NewShipSystem, "test-system", "user.NewShipSystem")
	})
}

// In order for 'go test' to run this suite, we need to create
// a normal test function and pass our suite to suite.Run
func TestDbTestSuite(t *testing.T) {
	suite.Run(t, new(DbTestSuite))
}
