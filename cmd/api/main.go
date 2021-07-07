package main

import (
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
)

func main() {
	s := NewServer()

	r := chi.NewRouter()

	// A good base middleware stack
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)

	// Set a timeout value on the request context (ctx), that will signal
	// through ctx.Done() that the request has timed out and further
	// processing should be stopped.
	r.Use(middleware.Timeout(60 * time.Second))

	r.Get("/", s.Index)
	r.Route("/api", func(r chi.Router) {
		// index
		r.Get("/", s.Index)

		// users
		r.Route("/users", func(r chi.Router) {
			// Users with latest stats
			r.Get("/", s.Index)
			// User with all stats
			r.Get("/{userId}", s.Index)
			// Users ships
			r.Get("/{userId}/ships", s.Index)
			// Users ship transactions
			r.Get("/{userId}/ships/{shipId}/transactions", s.Index)
		})

		// market data
		r.Get("/market-data/latest", s.Index)

		// systems
		r.Route("/systems", func(r chi.Router) {
			// Systems info
			r.Get("/", s.Index)

			// Systems goods
			r.Get("/{system}/goods", s.Index)

			// Systems routes per good
			r.Get("/{system}/routes/{good}", s.Index)
		})

		r.Route("/locations", func(r chi.Router) {
			r.Get("/{location}/goods", s.Index)

			r.Get("/{location}/market-data", s.Index)

			r.Get("/{location}/market-data/{good}", s.Index)

			r.Get("/{location}/routes", s.Index)
		})
	})

	fmt.Println("Listening on port 8080")
	log.Fatal(http.ListenAndServe(":8080", r))
}
