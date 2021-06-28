package spacemonger

import (
	"context"
	"log"
	"math"
	"time"

	"spacemonger/spacetraders"
)

type Ship struct {
	dbConn       DBConn
	user         User
	Id           string
	location     string
	ShipMessages chan ShipMessage
	Role         ShipRole
}

func NewShip(ctx context.Context, dbConn DBConn, u User, ship Ship) (Ship, error) {
	s, err := u.Client.GetMyShip(ctx, ship.Id)
	if err != nil {
		return Ship{}, err
	}

	return Ship{
		dbConn:       dbConn,
		user:         u,
		Id:           ship.Id,
		location:     s.Ship.Location,
		ShipMessages: u.ShipMessages,
		Role:         Scout,
	}, nil
}

func getSquaredDistance(a, b spacetraders.SystemLocation) float64 {
	return math.Pow(float64(b.X-a.X), 2) + math.Pow(float64(b.Y-a.Y), 2)
}

// https://www.geeksforgeeks.org/travelling-salesman-problem-implementation-using-backtracking/
func tsp(bestCost float64, bestPath []int, graph [][]float64, visited []bool, curPos int, cost float64, path []int) (float64, []int) {
	// If last node is reached and it has
	// a link to the starting node i.e
	// the source then keep the minimum
	// value out of the total cost of
	// traversal and "ans"
	// Finally return to check for
	// more possible values
	if len(path) == len(graph[curPos]) && (bestCost != 0 && cost >= bestCost) {
		log.Printf("Not new best cost %f, Not new best path %v\n", cost, path)
	}

	if len(path) == len(graph[curPos]) && (bestCost == 0 || cost < bestCost) {
		log.Printf("New best cost %f, New best path %v\n", cost, path)
		return cost, path
	}

	// BACKTRACKING STEP
	// Loop to traverse the adjacency list
	// of currPos node and increasing the count
	// by 1 and cost by graph[currPos][i] value
	for i, currentCost := range graph[curPos] {
		if visited[i] == false && currentCost != 0 {
			// Mark as visited
			log.Printf("%d -> %d Path: %v\n", curPos, i, append(path, i))
			visited[i] = true
			bestCost, bestPath = tsp(bestCost, bestPath, graph, visited, i, cost+currentCost, append(path, i))

			// Unmark i as visited pretty sure we don't need to unmark it since the visited array is copied into the subsequent
			// tsp calls and will be unmodified when this loop is run again
			visited[i] = false
		}
	}

	return bestCost, bestPath
}

func SortLocations(locations []spacetraders.SystemLocation) (float64, []string) {
	log.Printf("Locations length: %d\n", len(locations))
	visited := make([]bool, len(locations))
	visited[0] = true
	adjacencyList := make([][]float64, len(locations))

	for i, _ := range locations {
		for i2, _ := range locations {
			adjacencyList[i] = append(adjacencyList[i], getSquaredDistance(locations[i], locations[i2]))
		}
	}

	log.Printf("Visited: %+v\n", visited)
	log.Println("Adjacency List")
	for _, a := range adjacencyList {
		log.Printf("%v\n", a)
	}

	pathCost, pathIndexes := tsp(0, []int{}, adjacencyList, visited, 0, 0, []int{0})
	log.Printf("PathIndexes: %v\n", pathIndexes)
	var path []string
	for _, pathIndex := range pathIndexes {
		path = append(path, locations[pathIndex].Symbol)
	}

	return pathCost, path
}

func (s Ship) Run(ctx context.Context) <-chan error {
	exit := make(chan error)
	go func() {
		for {
			log.Printf("%s -- Collecting marketplace for location %s\n", s.user.Username, s.location)

			marketplace, err := s.user.Client.GetLocationMarketplace(ctx, s.location)
			if err != nil {
				exit <- err
				// TODO: return or continue
				return
			}

			if err := SaveLocationMarketplaceResponses(ctx, s.dbConn, s.location, marketplace); err != nil {
				log.Printf("%s -- Unable to collect marketplace data\n", s.user.Username)
				exit <- err
				// TODO: return or continue
				return
			}

			// Phase 1: Fill up on fuel and fly to each location collecting marketplace data

			locations, err := s.user.Client.GetSystemLocations(ctx, "OE")
			if err != nil {
				exit <- err
				// TODO: return or continue
				return
			}

			SortLocations(locations.Locations)

			log.Printf("%s -- Saved marketplace data for location %s\n", s.user.Username, s.location)

			// s.ShipMessages <- ShipMessage{
			// 	Type:       UpdateCredits,
			// 	NewCredits: 100000,
			// }

			time.Sleep(60 * time.Second)
		}
	}()

	return exit
}
