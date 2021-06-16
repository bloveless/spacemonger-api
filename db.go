package spacemonger

import (
	"context"
	"spacemonger/spacetrader"

	"github.com/jackc/pgx/v4/pgxpool"
)

func GetUser(ctx context.Context, pool *pgxpool.Pool, username string) (User, error) {
	u := User{}
	err := pool.QueryRow(ctx, `
		SELECT id::text, username, token, new_ship_assignment, new_ship_system FROM daemon_user
		WHERE username = $1
		LIMIT 1;
	`, username).Scan(&u.Id, &u.Username, &u.Token, &u.NewShipAssignment, &u.NewShipSystem)

	if err != nil {
		return User{}, err
	}

	return u, nil
}

func SaveUser(ctx context.Context, pool *pgxpool.Pool, user User) error {
	_, err := pool.Query(ctx, `
		INSERT INTO daemon_user (username, token, assignment, new_ship_assignment, new_ship_system)
		VALUES ($1, $2, $3, $4, $5);
	`, user.Username, user.Token, user.NewShipAssignment, user.NewShipSystem)

	return err
}

func GetLocation(ctx context.Context, pool *pgxpool.Pool, location string) (Location, error) {
	l := Location{}
	err := pool.QueryRow(ctx, `
		SELECT
			 system
			,system_name
			,location
			,location_name
			,location_type
			,x
			,y
			,created_at
		FROM daemon_location
		WHERE location = $1;
		`, location,
	).Scan(
		&l.System,
		&l.SystemName,
		&l.Location,
		&l.LocationName,
		&l.LocationType,
		&l.X,
		&l.Y,
		&l.CreatedAt,
	)

	if err != nil {
		return Location{}, err
	}

	return l, nil
}

func SaveLocation(ctx context.Context, pool *pgxpool.Pool, location Location) error {
	_, err := pool.Query(ctx, `
		INSERT INTO daemon_location (system, system_name, location, location_name, location_type, x, y)
		VALUES ($1, $2, $3, $4, $5, $6, $7);
		`,
		location.System,
		location.SystemName,
		location.Location,
		location.LocationName,
		location.LocationType,
		location.X,
		location.Y,
	)

	return err
}

func GetSystemLocationsFromLocation(ctx context.Context, pool *pgxpool.Pool, location string) ([]string, error) {
	rows, err := pool.Query(ctx, `
		SELECT
			dl1.location
		FROM daemon_location dl1
		INNER JOIN daemon_location dl2
			ON dl1.system = dl2.system
		WHERE dl2.location = $1;
	`, location)

	if err != nil {
		return []string{}, nil
	}

	var locations []string

	for rows.Next() {
		var location string

		err = rows.Scan(&location)
		if err != nil {
			return []string{}, nil
		}

		locations = append(locations, location)
	}

	return locations, nil
}

func SaveFlightPlan(ctx context.Context, pool *pgxpool.Pool, userId string, flightPlan spacetrader.FlightPlan) error {
	_, err := pool.Query(ctx, `
		INSERT INTO daemon_flight_plan (
			 id
			,user_id
			,ship_id
			,origin
			,destination
			,distance
			,fuel_consumed
			,fuel_remaining
			,time_remaining_in_seconds
			,arrives_at
		) VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10);
		`,
		flightPlan.Id,
		userId,
		flightPlan.ShipId,
		flightPlan.Departure,
		flightPlan.Destination,
		flightPlan.Distance,
		flightPlan.FuelConsumed,
		flightPlan.FuelRemaining,
		flightPlan.TimeRemainingInSeconds,
		flightPlan.ArrivesAt,
	)

	return err
}
