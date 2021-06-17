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
	"github.com/jackc/pgx/v4/pgxpool"
	"github.com/ory/dockertest/v3"
)

var pgpool *pgxpool.Pool

func TestMain(m *testing.M) {
	// uses a sensible default on windows (tcp/http) and linux/osx (socket)
	pool, err := dockertest.NewPool("")
	if err != nil {
		log.Fatalf("Could not connect to docker: %s", err)
	}

	wd, err := os.Getwd()
	if err != nil {
		log.Fatalf("Unable to determine working directory: %s", err)
	}

	// pulls an image, creates a container based on it and runs it
	resource, err := pool.RunWithOptions(&dockertest.RunOptions{
		Repository: "postgres",
		Tag:        "13",
		Env: []string{
			"POSTGRES_USER=spacemonger_test",
			"POSTGRES_DB=spacemonger_test",
			"POSTGRES_PASSWORD=Testing123",
		},
		Mounts: []string{
			fmt.Sprintf("%s/docker/postgres/initdb.d:/docker-entrypoint-initdb.d", filepath.Dir(wd)),
		},
	})
	if err != nil {
		log.Fatalf("Could not start resource: %s", err)
	}

	// exponential backoff-retry, because the application in the container might not be ready to accept connections yet
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := pool.Retry(func() error {
		var err error
		pgpool, err = pgxpool.Connect(ctx, fmt.Sprintf("postgres://spacemonger_test:Testing123@localhost:%s?sslmode=disable", resource.GetPort("5432/tcp")))
		if err != nil {
			return err
		}
		return pgpool.Ping(ctx)
	}); err != nil {
		log.Fatalf("Could not connect to docker: %s", err)
	}

	mig, err := migrate.New(fmt.Sprintf("file://%s/migrations", filepath.Dir(wd)), fmt.Sprintf("postgres://spacemonger_test:Testing123@localhost:%s?sslmode=disable", resource.GetPort("5432/tcp")))
	if err != nil {
		panic(err)
	}

	err = mig.Up()
	if err != nil && !errors.Is(err, migrate.ErrNoChange) {
		panic(err)
	}

	code := m.Run()

	// You can't defer this because os.Exit doesn't care for defer
	if err := pool.Purge(resource); err != nil {
		log.Fatalf("Could not purge resource: %s", err)
	}

	os.Exit(code)
}

func TestGetUser(t *testing.T) {
	t.Parallel()
	ctx, cancel := context.WithTimeout(context.Background(), 10 * time.Second)
	defer cancel()

	_, err := pgpool.Query(ctx, `
		INSERT INTO daemon_user (username, token, new_ship_assignment, new_ship_system)
		VALUES ('test-username', 'test-token', 'test-assignment', 'test-system');
	`)
	if err != nil {
		t.Fatal(err)
	}

	user, err := spacemonger.GetUser(ctx, pgpool, "test-username")
	if err != nil {
		t.Fatal(err)
	}

	stringNotBlank(t, "user.Id", user.Id)
	stringEquals(t, "user.Username", user.Username, "test-username")
	stringEquals(t, "user.Token", user.Token, "test-token")
	stringEquals(t, "user.NewShipAssignment", user.NewShipAssignment, "test-assignment")
	stringEquals(t, "user.NewShipSystem", user.NewShipSystem, "test-system")
}
