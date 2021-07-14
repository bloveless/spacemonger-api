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
	config         spacemonger.Config
	dbPool         *pgxpool.Pool
	userRepository spacemonger.UserRepository
	shipRepository spacemonger.ShipRepository
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

	return Server{
		config:         config,
		dbPool:         pool,
		userRepository: spacemonger.PostgresUserRepository{Conn: pool},
		shipRepository: spacemonger.PostgresShipRepository{Conn: pool},
	}
}

func (s *Server) Index(w http.ResponseWriter, r *http.Request) {
	_, err := w.Write([]byte("Hello world!"))
	if err != nil {
		log.Printf("unable to write message body: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUsers(w http.ResponseWriter, r *http.Request) {
	users, err := s.userRepository.GetAllUsersLatestStats(r.Context())
	if err != nil {
		log.Printf("unable to get users: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	err = json.NewEncoder(w).Encode(users)
	if err != nil {
		log.Printf("unable to encode users response: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUser(w http.ResponseWriter, r *http.Request) {
	userId := chi.URLParam(r, "userId")
	user, err := s.userRepository.GetUser(r.Context(), userId)
	if err != nil {
		log.Printf("unable to get user: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}

	err = json.NewEncoder(w).Encode(user)
	if err != nil {
		log.Printf("unable to encode user response: %s\n", err)
		http.Error(w, http.StatusText(500), http.StatusInternalServerError)
	}
}

func (s *Server) GetUserStats(w http.ResponseWriter, r *http.Request) {
	userId := chi.URLParam(r, "userId")
	userStats, err := s.userRepository.GetUserStats(r.Context(), userId)
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
	userShips, err := s.shipRepository.GetUserShips(r.Context(), userId)
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
