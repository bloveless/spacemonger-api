package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"

	"spacemonger"

	"github.com/go-chi/chi/v5"
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
	_, err := w.Write([]byte("Hello world!"))
	if err != nil {
		log.Printf("unable to write to response writer: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUsers(w http.ResponseWriter, r *http.Request) {
	users, err := spacemonger.GetUsersWithStats(r.Context(), s.dbPool)
	if err != nil {
		log.Printf("unable to get users: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	err = json.NewEncoder(w).Encode(users)
	if err != nil {
		log.Printf("unable to encode users: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUsersWithStats(w http.ResponseWriter, r *http.Request) {
	userId := chi.URLParam(r, "userId")
	userStats, err := spacemonger.GetUserStats(r.Context(), s.dbPool, userId)
	if err != nil {
		log.Printf("unable to get user stats: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	err = json.NewEncoder(w).Encode(userStats)
	if err != nil {
		log.Printf("unable to encode user stats: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUserShips(w http.ResponseWriter, r *http.Request) {
	userId := chi.URLParam(r, "userId")
	userShips, err := spacemonger.GetShips(r.Context(), s.dbPool, userId)
	if err != nil {
		log.Printf("unable to get user ships: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	err = json.NewEncoder(w).Encode(userShips)
	if err != nil {
		log.Printf("unable to encode user ships: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}
