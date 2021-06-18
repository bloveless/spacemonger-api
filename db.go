package spacemonger

import (
	"context"
	"spacemonger/spacetrader"

	"github.com/jackc/pgconn"
	"github.com/jackc/pgx/v4"
)

type DBConn interface {
	Exec(ctx context.Context, sql string, optionsAndArgs ...interface{}) (pgconn.CommandTag, error)
	Query(ctx context.Context, sql string, optionsAndArgs ...interface{}) (pgx.Rows, error)
	QueryRow(ctx context.Context, sql string, optionsAndArgs ...interface{}) pgx.Row
}

func GetUser(ctx context.Context, conn DBConn, username string) (User, error) {
	u := User{}
	err := conn.QueryRow(ctx, `
		SELECT id::text, username, token, new_ship_assignment, new_ship_system FROM daemon_user
		WHERE username = $1
		LIMIT 1;
	`, username).Scan(&u.Id, &u.Username, &u.Token, &u.NewShipAssignment, &u.NewShipSystem)

	if err != nil {
		return User{}, err
	}

	return u, nil
}

func SaveUser(ctx context.Context, conn DBConn, user User) error {
	_, err := conn.Query(ctx, `
		INSERT INTO daemon_user (username, token, assignment, new_ship_assignment, new_ship_system)
		VALUES ($1, $2, $3, $4, $5);
	`, user.Username, user.Token, user.NewShipAssignment, user.NewShipSystem)

	return err
}

func GetLocation(ctx context.Context, conn DBConn, location string) (Location, error) {
	l := Location{}
	err := conn.QueryRow(ctx, `
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

func SaveLocation(ctx context.Context, conn DBConn, location Location) error {
	_, err := conn.Query(ctx, `
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

func GetSystemLocationsFromLocation(ctx context.Context, conn DBConn, location string) ([]string, error) {
	rows, err := conn.Query(ctx, `
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

func SaveFlightPlan(ctx context.Context, conn DBConn, userId string, flightPlan spacetrader.FlightPlan) error {
	_, err := conn.Query(ctx, `
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

func GetActiveFlightPlan(ctx context.Context, conn DBConn, shipId string) (FlightPlan, error) {
	r := FlightPlan{}
	err := conn.QueryRow(ctx, `
		SELECT
			 id
			,ship_id
			,origin
			,destination
			,fuel_consumed
			,fuel_remaining
			,time_remaining_in_seconds
			,created_at
			,distance
			,arrives_at
			,user_id
		FROM daemon_flight_plan
		WHERE ship_id = $1
			AND arrives_at > $2
		`,
		shipId,
	).Scan(&r.Id, &r.ShipId, &r.Origin, &r.Destination, &r.FuelConsumed, &r.FuelRemaining, &r.TimeRemainingInSeconds, &r.CreatedAt, &r.Distance, &r.ArrivesAt, &r.UserId)
	if err != nil {
		return FlightPlan{}, err
	}

	return r, nil
}

func GetDistanceBetweenLocations(ctx context.Context, conn DBConn, origin, destination string) (DistanceBetweenLocations, error) {
	r := DistanceBetweenLocations{}
	err := conn.QueryRow(ctx, `
		SELECT
			 dsi1.location_type as origin_location_type
			,SQRT(POW(dsi1.x - dsi2.x, 2) + POW(dsi1.y - dsi2.y, 2)) AS distance
		FROM daemon_system_info dsi1
		INNER JOIN daemon_system_info dsi2
			-- for now we are going to restrict this to the same system since we don't have
			-- multiple stops built yet
			ON dsi1.system = dsi2.system
		WHERE dsi1.location = $1
			AND dsi2.location = $2;
		`,
		origin,
		destination,
	).Scan(&r.originLocationType, &r.distance)

	if err != nil {
		return DistanceBetweenLocations{}, err
	}

	return r, nil
}

func SaveMarketData(ctx context.Context, conn DBConn, location string, marketData spacetrader.MarketplaceData) error {
	_, err := conn.Query(ctx, `
		INSERT INTO daemon_market_data(location, good, purchase_price_per_unit, sell_price_per_unit, volume_per_unit, quantity_available)
		VALUES ($1, $2, $3, $4, $5, $6);
		`,
		location,
		marketData.Good,
		marketData.VolumePerUnit,
		marketData.QuantityAvailable,
		marketData.PurchasePricePerUnit,
		marketData.SellPricePerUnit,
	)
	if err != nil {
		return err
	}

	return nil
}

func GetRoutesFromLocation(ctx context.Context, conn DBConn, location string, shipSpeed int) ([]Route, error) {
	r := []Route{}

	_, err := conn.Exec(ctx, `DROP TABLE IF EXISTS tmp_latest_location_goods`)
	if err != nil {
		return []Route{}, err
	}

	_, err = conn.Exec(ctx, `
		CREATE TEMPORARY TABLE tmp_latest_location_goods (
			 location VARCHAR(100) NOT NULL
			,location_type VARCHAR(100) NOT NULL
			,x INT NOT NULL
			,y INT NOT NULL
			,good VARCHAR(100) NOT NULL
			,purchase_price_per_unit INT NOT NULL
			,sell_price_per_unit INT NOT NULL
			,volume_per_unit INT NOT NULL
			,quantity_available INT NOT NULL
			,created_at TIMESTAMP WITH TIME ZONE NOT NULL
		);
	`)
	if err != nil {
		return []Route{}, err
	}

	_, err = conn.Exec(ctx, `
		-- Get the latest market data from each good in each location
		WITH ranked_location_goods AS (
			SELECT
				 id
				,ROW_NUMBER() OVER (
					PARTITION BY location, good
					ORDER BY created_at DESC
				) AS rank
			FROM daemon_market_data
		)
		INSERT INTO tmp_latest_location_goods (
			 location
			,location_type
			,x
			,y
			,good
			,purchase_price_per_unit
			,sell_price_per_unit
			,volume_per_unit
			,quantity_available
			,created_at
		)
		SELECT
			 dmd.location
			,dsi.location_type
			,dsi.x
			,dsi.y
			,dmd.good
			,dmd.purchase_price_per_unit
			,dmd.sell_price_per_unit
			,dmd.volume_per_unit
			,dmd.quantity_available
			,dmd.created_at
		FROM daemon_market_data dmd
		INNER JOIN ranked_location_goods rlg ON dmd.id = rlg.id
		INNER JOIN daemon_system_info dsi on dmd.location = dsi.location
		WHERE rlg.rank = 1
			AND dmd.created_at > (now() at time zone 'utc' - INTERVAL '30 min')
		ORDER BY dmd.good, dmd.location;
	`)
	if err != nil {
		return []Route{}, err
	}

	rows, err := conn.Query(ctx, `
		-- calculate the route from each location to each location per good
		-- limited to routes which will actually turn a profit
		SELECT
			 llg1.location AS purchase_location
			,llg1.location_type AS purchase_location_type
			,llg2.location AS sell_location
			,llg2.good
			,SQRT(POW(llg1.x - llg2.x, 2) + POW(llg2.y - llg1.y, 2)) AS distance
			,llg1.quantity_available AS purchase_quantity
			,llg2.quantity_available AS sell_quantity
			,llg1.purchase_price_per_unit AS purchase_price_per_unit
			,llg2.sell_price_per_unit AS sell_price_per_unit
			,llg1.volume_per_unit AS volume_per_unit
		FROM tmp_latest_location_goods llg1
		CROSS JOIN tmp_latest_location_goods llg2
		INNER JOIN daemon_system_info from_dsi
			ON from_dsi.location = llg1.location
		INNER JOIN daemon_system_info to_dsi
			ON to_dsi.location = llg2.location
		WHERE from_dsi.location = $1
			AND from_dsi.system = to_dsi.system
			AND llg1.good = llg2.good
			AND llg1.location != llg2.location
		`,
		location,
	)

	return r, nil
}
