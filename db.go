package spacemonger

import (
	"context"
	"fmt"
	"log"
	"time"

	"spacemonger/spacetraders"

	"github.com/jackc/pgconn"
	"github.com/jackc/pgx/v4"
)

type DbConn interface {
	Exec(ctx context.Context, sql string, optionsAndArgs ...interface{}) (pgconn.CommandTag, error)
	Query(ctx context.Context, sql string, optionsAndArgs ...interface{}) (pgx.Rows, error)
	QueryRow(ctx context.Context, sql string, optionsAndArgs ...interface{}) pgx.Row
}

func GetUsers(ctx context.Context, conn DbConn) ([]User, error) {
	var users []User
	rows, err := conn.Query(ctx, `SELECT id::text, username, token, new_ship_role_data FROM daemon_user`)
	if err != nil {
		return nil, fmt.Errorf("unable to get users from db: %w", err)
	}

	for rows.Next() {
		u := User{}

		rows.Scan(&u.Id, &u.Username, &u.Token, &u.NewShipRoleData)

		users = append(users, u)
	}

	return users, nil
}

func GetUser(ctx context.Context, conn DbConn, username string) (User, error) {
	u := User{}
	err := conn.QueryRow(ctx, `
		SELECT id::text, username, token, new_ship_role_data FROM daemon_user
		WHERE username = $1
		LIMIT 1;
		`,
		username,
	).Scan(&u.Id, &u.Username, &u.Token, &u.NewShipRoleData)

	if err != nil {
		return User{}, err
	}

	return u, nil
}

// SaveUser saves the user to the DB and returns a new user with the Id field populated
func SaveUser(ctx context.Context, conn DbConn, user User) (User, error) {
	err := conn.QueryRow(ctx, `
		INSERT INTO daemon_user (username, token, new_ship_role_data)
		VALUES ($1, $2, $3)
		RETURNING id;
		`,
		user.Username,
		user.Token,
		user.NewShipRoleData,
	).Scan(&user.Id)

	return user, err
}

func GetShip(ctx context.Context, conn DbConn, shipId string) (DbShip, error) {
	s := DbShip{}

	err := conn.QueryRow(ctx, `
		SELECT
			 user_id
			,ship_id
			,type
			,class
			,max_cargo
			,loading_speed
			,speed
			,manufacturer
			,plating
			,weapons
			,role_data
			,location
		FROM daemon_user_ship
		WHERE ship_id = $1;
		`,
		shipId,
	).Scan(
		&s.UserId,
		&s.ShipId,
		&s.Type,
		&s.Class,
		&s.MaxCargo,
		&s.LoadingSpeed,
		&s.Speed,
		&s.Manufacturer,
		&s.Plating,
		&s.Weapons,
		&s.RoleData,
		&s.Location,
	)
	if err != nil {
		return DbShip{}, err
	}

	return s, nil
}

func GetShips(ctx context.Context, conn DbConn, userId string) ([]DbShip, error) {
	rows, err := conn.Query(ctx, `
		SELECT
			 user_id
			,ship_id
			,type
			,class
			,max_cargo
			,loading_speed
			,speed
			,manufacturer
			,plating
			,weapons
			,role_data
			,location
		FROM daemon_user_ship
		WHERE user_id = $1;
		`,
		userId,
	)
	if err != nil {
		return []DbShip{}, err
	}

	var ships []DbShip
	for rows.Next() {
		s := DbShip{}

		err := rows.Scan(
			&s.UserId,
			&s.ShipId,
			&s.Type,
			&s.Class,
			&s.MaxCargo,
			&s.LoadingSpeed,
			&s.Speed,
			&s.Manufacturer,
			&s.Plating,
			&s.Weapons,
			&s.RoleData,
			&s.Location,
		)
		if err != nil {
			return []DbShip{}, err
		}

		ships = append(ships, s)
	}

	return ships, nil
}

func UpdateShipLocation(ctx context.Context, conn DbConn, s Ship, location string) error {
	_, err := conn.Exec(ctx, `
		UPDATE daemon_user_ship
		SET location = $2
		WHERE ship_id = $1
		`,
		s.Id,
		location,
	)
	if err != nil {
		return err
	}

	return nil
}

func SaveSystem(ctx context.Context, conn DbConn, system spacetraders.GetSystemResponse) error {
	_, err := conn.Exec(ctx, `
		INSERT INTO daemon_system(system, name) VALUES ($1, $2)
		ON CONFLICT (system)
		DO UPDATE
			SET name = $2;
		`,
		system.System.Symbol,
		system.System.Name,
	)

	return err
}

func GetLocation(ctx context.Context, conn DbConn, location string) (DbLocation, error) {
	l := DbLocation{}
	err := conn.QueryRow(ctx, `
		SELECT
			 system
			,location
			,location_name
			,location_type
			,x
			,y
			,created_at
		FROM daemon_location
		WHERE location = $1;
		`,
		location,
	).Scan(
		&l.System,
		&l.Location,
		&l.LocationName,
		&l.Type,
		&l.X,
		&l.Y,
		&l.CreatedAt,
	)

	if err != nil {
		return DbLocation{}, err
	}

	return l, nil
}

func SaveLocation(ctx context.Context, conn DbConn, location DbLocation) error {
	_, err := conn.Exec(ctx, `
		INSERT INTO daemon_location (system, location, location_name, location_type, x, y)
		VALUES ($1, $2, $3, $4, $5, $6)
		ON CONFLICT (system, location)
		DO UPDATE
			SET location_name = $3,
				location_type = $4,
				x = $5,
				y = $6;
		`,
		location.System,
		location.Location,
		location.LocationName,
		location.Type,
		location.X,
		location.Y,
	)

	return err
}

func GetSystemLocationsFromLocation(ctx context.Context, conn DbConn, location string) ([]string, error) {
	rows, err := conn.Query(ctx, `
		SELECT
			dl1.location
		FROM daemon_location dl1
		INNER JOIN daemon_location dl2
			ON dl1.system = dl2.system
		WHERE dl2.location = $1;
		`,
		location,
	)
	if err != nil {
		return []string{}, nil
	}
	defer rows.Close()

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

func SaveFlightPlan(ctx context.Context, conn DbConn, userId string, flightPlan DbFlightPlan) error {
	_, err := conn.Exec(ctx, `
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
			,created_at
		) VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11);
		`,
		flightPlan.Id,
		userId,
		flightPlan.ShipId,
		flightPlan.Origin,
		flightPlan.Destination,
		flightPlan.Distance,
		flightPlan.FuelConsumed,
		flightPlan.FuelRemaining,
		flightPlan.TimeRemainingInSeconds,
		flightPlan.ArrivesAt,
		flightPlan.CreatedAt,
	)

	return err
}

func GetActiveFlightPlan(ctx context.Context, conn DbConn, shipId string) (DbFlightPlan, error) {
	r := DbFlightPlan{}
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
		time.Now(),
	).Scan(&r.Id, &r.ShipId, &r.Origin, &r.Destination, &r.FuelConsumed, &r.FuelRemaining, &r.TimeRemainingInSeconds, &r.CreatedAt, &r.Distance, &r.ArrivesAt, &r.UserId)
	if err != nil {
		return DbFlightPlan{}, err
	}

	return r, nil
}

func GetDistanceBetweenLocations(ctx context.Context, conn DbConn, origin, destination string) (DbDistanceBetweenLocations, error) {
	r := DbDistanceBetweenLocations{}
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
		return DbDistanceBetweenLocations{}, err
	}

	return r, nil
}

func SaveLocationMarketplaceResponses(ctx context.Context, conn DbConn, location string, marketplaceData spacetraders.GetLocationMarketplaceResponse) error {
	for _, m := range marketplaceData.Marketplace {
		_, err := conn.Exec(ctx, `
			INSERT INTO daemon_marketplace(location, good, purchase_price_per_unit, sell_price_per_unit, volume_per_unit, quantity_available)
			VALUES ($1, $2, $3, $4, $5, $6);
			`,
			location,
			m.Good,
			m.PurchasePricePerUnit,
			m.SellPricePerUnit,
			m.VolumePerUnit,
			m.QuantityAvailable,
		)

		if err != nil {
			return err
		}

		// TODO: It is possible that a good disappears completely from a location... how often does this happen... if ever
		//       I can take care of it but I'm curious if it matters that much but since the marketplace data is processed
		//       one row at a time there would need to be a more significant change to fix it. So I'm going to ignore it
		//       until it actually becomes an issue

		_, err = conn.Exec(ctx, `
			INSERT INTO daemon_marketplace_latest(location, good, purchase_price_per_unit, sell_price_per_unit, volume_per_unit, quantity_available)
			VALUES ($1, $2, $3, $4, $5, $6)
			ON CONFLICT (location, good)
			DO UPDATE
				SET purchase_price_per_unit = $3,
					sell_price_per_unit = $4,
					volume_per_unit = $5,
					quantity_available = $6;
			`,
			location,
			m.Good,
			m.PurchasePricePerUnit,
			m.SellPricePerUnit,
			m.VolumePerUnit,
			m.QuantityAvailable,
		)

		if err != nil {
			return err
		}
	}

	return nil
}

func GetRoutesFromLocation(ctx context.Context, conn DbConn, location string) ([]DbRoute, error) {
	var routes []DbRoute

	rows, err := conn.Query(ctx, `
		-- calculate the route from each location to each location per good
		SELECT
			 dml1.location AS purchase_location
			,from_dl.location_type AS purchase_location_type
			,dml2.location AS sell_location
			,dml2.good
			,SQRT(POW(from_dl.x - to_dl.x, 2) + POW(from_dl.y - to_dl.y, 2)) AS distance
			,dml1.quantity_available AS purchase_quantity_available
			,dml2.quantity_available AS sell_quantity_available
			,dml1.purchase_price_per_unit AS purchase_price_per_unit
			,dml2.sell_price_per_unit AS sell_price_per_unit
			,dml1.volume_per_unit AS volume_per_unit
		FROM daemon_marketplace_latest dml1
		CROSS JOIN daemon_marketplace_latest dml2
		INNER JOIN daemon_location from_dl
			ON from_dl.location = dml1.location
		INNER JOIN daemon_location to_dl
			ON to_dl.location = dml2.location
		WHERE dml1.location = $1
			AND from_dl.system = to_dl.system
			AND dml1.good = dml2.good
			AND dml1.location != dml2.location
		`,
		location,
	)
	if err != nil {
		return []DbRoute{}, err
	}
	defer rows.Close()

	for rows.Next() {
		r := DbRoute{}
		err = rows.Scan(
			&r.PurchaseLocation,
			&r.PurchaseLocationType,
			&r.SellLocation,
			&r.Good,
			&r.Distance,
			&r.PurchaseLocationQuantity,
			&r.SellLocationQuantity,
			&r.PurchasePricePerUnit,
			&r.SellPricePerUnit,
			&r.VolumePerUnit,
		)
		if err != nil {
			return []DbRoute{}, err
		}

		routes = append(routes, r)
	}

	return routes, nil
}

func SaveShip(ctx context.Context, conn DbConn, user User, ship DbShip) error {
	_, err := conn.Exec(ctx, `
		INSERT INTO daemon_user_ship (
			 user_id
			,ship_id
			,type
			,class
			,max_cargo
			,loading_speed
			,speed
			,manufacturer
			,plating
			,weapons
			,role_data
			,location
		) VALUES (
			 $1::uuid
			,$2
			,$3
			,$4
			,$5
			,$6
			,$7
			,$8
			,$9
			,$10
			,$11
			,$12
		)
		ON CONFLICT (user_id, ship_id)
		DO UPDATE SET
			 type = $3
			,class = $4
			,max_cargo = $5
			,loading_speed = $6
			,speed = $7
			,manufacturer = $8
			,plating = $9
			,weapons = $10
			-- Don't update role data on conflict. This way if a ship is re-assigned something other than the default
			-- after its initial create it will remain that way but new ships will receive the default assignment
			,location = $12
			,modified_at = timezone('utc', NOW());
		`,
		user.Id,
		ship.ShipId,
		ship.Type,
		ship.Class,
		ship.MaxCargo,
		ship.LoadingSpeed,
		ship.Speed,
		ship.Manufacturer,
		ship.Plating,
		ship.Weapons,
		ship.RoleData,
		ship.Location,
	)

	if err != nil {
		return err
	}

	return nil
}

func GetFuelRequired(ctx context.Context, conn DbConn, origin string, destination string, shipType string) (int, error) {
	fuelRequired := 0

	err := conn.QueryRow(ctx, `
		SELECT fuel_consumed
        FROM daemon_flight_plan dfp
        INNER JOIN daemon_user_ship dus
            ON dus.ship_id = dfp.ship_id
        WHERE dfp.origin = $1
            AND dfp.destination = $2
            AND dus.type = $3
        LIMIT 1
		`,
		origin,
		destination,
		shipType,
	).Scan(&fuelRequired)
	if err != nil {
		return 0, err
	}

	return fuelRequired, nil
}

func SaveUserStats(ctx context.Context, conn DbConn, u User) error {
	log.Printf("%s -- User Credits %d\n", u.Username, u.Credits)
	log.Printf("%s -- User Ships Length %d\n", u.Username, len(u.Ships))

	type ship struct {
		Id           string   `json:"id"`
		Type         string   `json:"type"`
		Location     string   `json:"location"`
		LoadingSpeed int      `json:"loading_speed"`
		MaxCargo     int      `json:"max_cargo"`
		Cargo        []Cargo  `json:"cargo"`
		RoleData     RoleData `json:"role_data"`
	}

	var ships []ship
	for _, s := range u.Ships {
		ships = append(ships, ship{
			Id:           s.Id,
			Type:         s.Type,
			Location:     s.Location,
			LoadingSpeed: s.LoadingSpeed,
			MaxCargo:     s.MaxCargo,
			Cargo:        s.Cargo,
			RoleData:     s.RoleData,
		})
	}

	_, err := conn.Exec(ctx, `
		INSERT INTO daemon_user_stats (user_id, credits, ship_count, ships)
		VALUES ($1::uuid, $2, $3, $4)
		`,
		u.Id,
		u.Credits,
		len(ships),
		ships,
	)
	if err != nil {
		return err
	}

	return nil
}

func SaveTransaction(ctx context.Context, conn DbConn, transaction DbTransaction) error {
	_, err := conn.Exec(ctx, `
		INSERT INTO daemon_user_transaction (
             user_id
            ,ship_id
            ,type
            ,good
            ,price_per_unit
            ,quantity
            ,total
            ,location
        ) VALUES (
             $1::uuid
            ,$2
            ,$3
            ,$4
            ,$5
            ,$6
            ,$7
            ,$8
        )
		`,
		transaction.UserId,
		transaction.ShipId,
		transaction.Type,
		transaction.Good,
		transaction.PricePerUnit,
		transaction.Quantity,
		transaction.Total,
		transaction.Location,
	)
	if err != nil {
		return fmt.Errorf("unable to save transaction: %w", err)
	}

	return nil
}
