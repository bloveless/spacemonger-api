package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"spacemonger"

	"github.com/jackc/pgx/v4/pgxpool"
)

type Server struct {
	config spacemonger.Config
	dbPool *pgxpool.Pool
}

func NewServer() Server {
	config, err := spacemonger.LoadConfig()
	if err != nil {
		log.Fatalf("Unable to load app config: %s", err)
	}

	pool, err := pgxpool.Connect(context.Background(), config.PostgresUrl)
	if err != nil {
		log.Fatalf("Unable to connect to connect to database: %s", err)
	}

	return Server{dbPool: pool, config: config}
}

func (s *Server) Index(w http.ResponseWriter, r *http.Request) {
	w.Write([]byte("Hello world!"))
}

func (s *Server) GetUsers(w http.ResponseWriter, r *http.Request) {
	users, err := spacemonger.GetUsers(r.Context(), s.dbPool)
	if err != nil {
		log.Printf("unable to get users: %s", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	json.NewEncoder(w).Encode(users)
}
